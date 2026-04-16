use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{
    AppHandle, Emitter, Manager, Runtime, State, Webview, WebviewUrl, WebviewWindowBuilder,
};

#[derive(Default)]
struct StartupFiles {
    pending: Mutex<Vec<OpenDocumentResult>>,
    per_window: Mutex<HashMap<String, Vec<OpenDocumentResult>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenDocumentResult {
    file_name: String,
    file_path: Option<String>,
    data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveDocumentRequest {
    mode: String,
    file_name: String,
    file_path: Option<String>,
    format: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveDocumentResult {
    file_name: String,
    file_path: Option<String>,
    format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecentDocument {
    name: String,
    path: Option<String>,
    format: String,
    source: String,
    last_opened_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileAssociationStatus {
    supported: bool,
    is_default: bool,
    message: String,
    default_app_hwp: Option<String>,
    default_app_hwpx: Option<String>,
    pending_mime_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecoverySnapshotMeta {
    id: String,
    file_name: String,
    file_path: Option<String>,
    format: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecoverySnapshotPayload {
    id: String,
    file_name: String,
    file_path: Option<String>,
    format: String,
    updated_at: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriteRecoverySnapshotRequest {
    snapshot_id: Option<String>,
    file_name: String,
    file_path: Option<String>,
    format: String,
    bytes: Vec<u8>,
}

fn normalize_file_name(file_name: &str, format: &str) -> String {
    let extension = if format.eq_ignore_ascii_case("hwpx") {
        ".hwpx"
    } else {
        ".hwp"
    };

    if file_name.to_ascii_lowercase().ends_with(extension) {
        file_name.to_string()
    } else {
        let trimmed = file_name.trim_end_matches(".hwp").trim_end_matches(".hwpx");
        format!("{trimmed}{extension}")
    }
}

fn format_from_path(path: &Path) -> String {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "hwpx" => "hwpx".to_string(),
        _ => "hwp".to_string(),
    }
}

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|err| err.to_string())?;
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    Ok(dir)
}

fn recents_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("recent-documents.json"))
}

fn recovery_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?.join("recovery");
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    Ok(dir)
}

fn recovery_meta_path(dir: &Path, snapshot_id: &str) -> PathBuf {
    dir.join(format!("{snapshot_id}.json"))
}

fn recovery_data_path(dir: &Path, snapshot_id: &str) -> PathBuf {
    dir.join(format!("{snapshot_id}.bin"))
}

fn load_recent_documents(app: &AppHandle) -> Result<Vec<RecentDocument>, String> {
    let path = recents_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&content).map_err(|err| err.to_string())
}

fn store_recent_documents(app: &AppHandle, docs: &[RecentDocument]) -> Result<(), String> {
    let path = recents_path(app)?;
    let payload = serde_json::to_string_pretty(docs).map_err(|err| err.to_string())?;
    fs::write(path, payload).map_err(|err| err.to_string())
}

fn remember_recent_document(app: &AppHandle, path: &Path, format: &str) -> Result<(), String> {
    let mut docs = load_recent_documents(app)?;
    let path_string = path.to_string_lossy().to_string();
    docs.retain(|doc| doc.path.as_deref() != Some(path_string.as_str()));
    docs.insert(
        0,
        RecentDocument {
            name: path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("document")
                .to_string(),
            path: Some(path_string),
            format: format.to_string(),
            source: "desktop".to_string(),
            last_opened_at: chrono::Utc::now().to_rfc3339(),
        },
    );
    docs.truncate(10);
    store_recent_documents(app, &docs)
}

fn open_path(path: &Path) -> Result<OpenDocumentResult, String> {
    let data = fs::read(path).map_err(|err| err.to_string())?;
    Ok(OpenDocumentResult {
        file_name: path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("document")
            .to_string(),
        file_path: Some(path.to_string_lossy().to_string()),
        data,
    })
}

fn collect_startup_files() -> Vec<OpenDocumentResult> {
    std::env::args_os()
        .skip(1)
        .filter_map(|arg| {
            let path = PathBuf::from(arg);
            let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
            if ext != "hwp" && ext != "hwpx" {
                return None;
            }
            open_path(&path).ok()
        })
        .collect()
}

fn next_snapshot_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("snapshot-{}-{}", std::process::id(), nanos)
}

fn load_recovery_snapshot_meta(path: &Path) -> Result<RecoverySnapshotMeta, String> {
    let content = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&content).map_err(|err| err.to_string())
}

fn load_file_association_status() -> Result<FileAssociationStatus, String> {
    #[cfg(target_os = "linux")]
    {
        let default_hwp = query_default_app("application/x-hwp")?;
        let default_hwpx = query_default_app("application/x-hwpx")?;
        let mut pending = Vec::new();

        if default_hwp.as_deref() != Some("rhwp.desktop") {
            pending.push("application/x-hwp".to_string());
        }
        if default_hwpx.as_deref() != Some("rhwp.desktop") {
            pending.push("application/x-hwpx".to_string());
        }

        let is_default = pending.is_empty();
        return Ok(FileAssociationStatus {
            supported: true,
            is_default,
            message: if is_default {
                "rhwp is already the default app for HWP and HWPX files.".to_string()
            } else {
                "Set rhwp as the default app to open HWP and HWPX files by double click."
                    .to_string()
            },
            default_app_hwp: default_hwp,
            default_app_hwpx: default_hwpx,
            pending_mime_types: pending,
        });
    }

    #[allow(unreachable_code)]
    Ok(FileAssociationStatus {
        supported: false,
        is_default: false,
        message: "Desktop file association checks are supported on Linux only.".to_string(),
        default_app_hwp: None,
        default_app_hwpx: None,
        pending_mime_types: vec![
            "application/x-hwp".to_string(),
            "application/x-hwpx".to_string(),
        ],
    })
}

#[cfg(target_os = "linux")]
fn query_default_app(mime_type: &str) -> Result<Option<String>, String> {
    let output = std::process::Command::new("xdg-mime")
        .args(["query", "default", mime_type])
        .output()
        .map_err(|err| err.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        Ok(None)
    } else {
        Ok(Some(stdout))
    }
}

fn prepare_startup_windows(app: &AppHandle, startup: &State<StartupFiles>) -> Result<(), String> {
    let startup_files = {
        let mut guard = startup.pending.lock().map_err(|_| "startup files mutex".to_string())?;
        std::mem::take(&mut *guard)
    };

    if startup_files.is_empty() {
        return Ok(());
    }

    let mut per_window = startup
        .per_window
        .lock()
        .map_err(|_| "startup files map mutex".to_string())?;
    per_window.insert("main".to_string(), vec![startup_files[0].clone()]);

    for (index, file) in startup_files.into_iter().enumerate().skip(1) {
        let label = format!("document-{}", index + 1);
        per_window.insert(label.clone(), vec![file]);
        WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
            .title("rhwp")
            .inner_size(1440.0, 980.0)
            .min_inner_size(960.0, 720.0)
            .resizable(true)
            .build()
            .map_err(|err| err.to_string())?;
    }

    Ok(())
}

#[tauri::command]
fn open_document(app: AppHandle) -> Result<Option<OpenDocumentResult>, String> {
    let picked = rfd::FileDialog::new()
        .add_filter("HWP documents", &["hwp", "hwpx"])
        .pick_file();

    let Some(path) = picked else {
        return Ok(None);
    };

    let format = format_from_path(&path);
    remember_recent_document(&app, &path, &format)?;
    open_path(&path).map(Some)
}

#[tauri::command]
fn save_document(
    app: AppHandle,
    request: SaveDocumentRequest,
) -> Result<Option<SaveDocumentResult>, String> {
    let target_path = if request.mode == "save" {
        request.file_path.clone().map(PathBuf::from)
    } else {
        None
    };

    let selected_path = match target_path {
        Some(path) => path,
        None => {
            let suggested_name = normalize_file_name(&request.file_name, &request.format);
            let picked = rfd::FileDialog::new()
                .add_filter("HWP documents", &["hwp", "hwpx"])
                .set_file_name(&suggested_name)
                .save_file();

            let Some(path) = picked else {
                return Ok(None);
            };
            path
        }
    };

    if let Some(parent) = selected_path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }

    fs::write(&selected_path, &request.bytes).map_err(|err| err.to_string())?;
    remember_recent_document(&app, &selected_path, &request.format)?;

    Ok(Some(SaveDocumentResult {
        file_name: selected_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("document")
            .to_string(),
        file_path: Some(selected_path.to_string_lossy().to_string()),
        format: request.format,
    }))
}

#[tauri::command]
fn get_recent_documents(app: AppHandle) -> Result<Vec<RecentDocument>, String> {
    load_recent_documents(&app)
}

#[tauri::command]
fn get_file_association_status() -> Result<FileAssociationStatus, String> {
    load_file_association_status()
}

#[tauri::command]
fn set_default_file_association() -> Result<FileAssociationStatus, String> {
    #[cfg(target_os = "linux")]
    {
        for mime_type in ["application/x-hwp", "application/x-hwpx"] {
            let status = std::process::Command::new("xdg-mime")
                .args(["default", "rhwp.desktop", mime_type])
                .status()
                .map_err(|err| err.to_string())?;
            if !status.success() {
                return Err(format!("xdg-mime failed for {}", mime_type));
            }
        }
    }

    load_file_association_status()
}

#[tauri::command]
fn list_recovery_snapshots(app: AppHandle) -> Result<Vec<RecoverySnapshotMeta>, String> {
    let dir = recovery_dir(&app)?;
    let mut snapshots = Vec::new();

    for entry in fs::read_dir(dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        if let Ok(meta) = load_recovery_snapshot_meta(&path) {
            snapshots.push(meta);
        }
    }

    snapshots.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(snapshots)
}

#[tauri::command]
fn read_recovery_snapshot(
    app: AppHandle,
    snapshot_id: String,
) -> Result<RecoverySnapshotPayload, String> {
    let dir = recovery_dir(&app)?;
    let meta = load_recovery_snapshot_meta(&recovery_meta_path(&dir, &snapshot_id))?;
    let data = fs::read(recovery_data_path(&dir, &snapshot_id)).map_err(|err| err.to_string())?;

    Ok(RecoverySnapshotPayload {
        id: meta.id,
        file_name: meta.file_name,
        file_path: meta.file_path,
        format: meta.format,
        updated_at: meta.updated_at,
        bytes: data,
    })
}

#[tauri::command]
fn write_recovery_snapshot(
    app: AppHandle,
    request: WriteRecoverySnapshotRequest,
) -> Result<RecoverySnapshotMeta, String> {
    let dir = recovery_dir(&app)?;
    let snapshot_id = request.snapshot_id.unwrap_or_else(next_snapshot_id);
    let meta = RecoverySnapshotMeta {
        id: snapshot_id.clone(),
        file_name: request.file_name,
        file_path: request.file_path,
        format: request.format,
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    fs::write(recovery_data_path(&dir, &snapshot_id), request.bytes).map_err(|err| err.to_string())?;
    fs::write(
        recovery_meta_path(&dir, &snapshot_id),
        serde_json::to_vec_pretty(&meta).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())?;

    Ok(meta)
}

#[tauri::command]
fn delete_recovery_snapshot(app: AppHandle, snapshot_id: String) -> Result<(), String> {
    let dir = recovery_dir(&app)?;
    let meta_path = recovery_meta_path(&dir, &snapshot_id);
    let data_path = recovery_data_path(&dir, &snapshot_id);

    if meta_path.exists() {
        fs::remove_file(meta_path).map_err(|err| err.to_string())?;
    }
    if data_path.exists() {
        fs::remove_file(data_path).map_err(|err| err.to_string())?;
    }

    Ok(())
}

#[tauri::command]
fn reveal_in_folder(path: String) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let target = PathBuf::from(path);
        let parent = target.parent().unwrap_or(target.as_path());
        std::process::Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|err| err.to_string())?;
        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    let _ = &path;

    #[allow(unreachable_code)]
    Ok(())
}

fn emit_startup_files<R: Runtime>(window: &Webview<R>, startup: &State<StartupFiles>) {
    let payload = {
        let mut guard = startup.per_window.lock().expect("startup files map mutex");
        guard.remove(window.label()).unwrap_or_default()
    };

    if payload.is_empty() {
        return;
    }

    let _ = window.emit("rhwp://open-files", payload);
}

fn main() {
    tauri::Builder::default()
        .manage(StartupFiles {
            pending: Mutex::new(collect_startup_files()),
            per_window: Mutex::new(HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![
            open_document,
            save_document,
            get_recent_documents,
            get_file_association_status,
            set_default_file_association,
            list_recovery_snapshots,
            read_recovery_snapshot,
            write_recovery_snapshot,
            delete_recovery_snapshot,
            reveal_in_folder
        ])
        .setup(|app| {
            let startup = app.state::<StartupFiles>();
            prepare_startup_windows(app.handle(), &startup)?;
            Ok(())
        })
        .on_page_load(|window, _| {
            emit_startup_files(window, &window.state::<StartupFiles>());
        })
        .run(tauri::generate_context!())
        .expect("error while running rhwp-desktop");
}
