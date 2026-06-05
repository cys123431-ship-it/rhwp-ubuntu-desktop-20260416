#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{
    AppHandle, DragDropEvent, Emitter, Manager, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, Window, WindowEvent,
};

#[cfg(target_os = "windows")]
use winreg::{
    enums::{HKEY_CLASSES_ROOT, HKEY_CURRENT_USER},
    RegKey,
};

const HWP_MIME: &str = "application/x-hwp";
const HWPX_MIME: &str = "application/vnd.hancom.hwpx";
#[cfg(target_os = "linux")]
const HWPX_LEGACY_MIME: &str = "application/x-hwpx";

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
struct SaveBinaryRequest {
    suggested_name: String,
    file_path: Option<String>,
    format: String,
    mime_type: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveBinaryResult {
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
    platform: String,
    action_mode: String,
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

fn binary_extension_for_format(format: &str) -> &'static str {
    match format.to_ascii_lowercase().as_str() {
        "hwpx" => ".hwpx",
        "pdf" => ".pdf",
        "docx" => ".docx",
        "jpg" | "jpeg" => ".jpg",
        _ => ".hwp",
    }
}

fn document_extension_for_format(format: &str) -> &'static str {
    if format.eq_ignore_ascii_case("hwpx") {
        ".hwpx"
    } else {
        ".hwp"
    }
}

fn normalize_path_extension(path: PathBuf, extension: &str) -> PathBuf {
    let target = extension.trim_start_matches('.');
    if path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case(target))
        .unwrap_or(false)
    {
        return path;
    }

    let mut normalized = path;
    normalized.set_extension(target);
    normalized
}

fn normalize_binary_name(file_name: &str, format: &str) -> String {
    let extension = binary_extension_for_format(format);

    if file_name.to_ascii_lowercase().ends_with(extension) {
        file_name.to_string()
    } else {
        let trimmed = file_name
            .trim_end_matches(".hwp")
            .trim_end_matches(".hwpx")
            .trim_end_matches(".pdf")
            .trim_end_matches(".docx")
            .trim_end_matches(".jpg")
            .trim_end_matches(".jpeg");
        format!("{trimmed}{extension}")
    }
}

fn save_dialog_for_binary_format(dialog: rfd::FileDialog, format: &str) -> rfd::FileDialog {
    match format.to_ascii_lowercase().as_str() {
        "hwpx" => dialog.add_filter("HWPX documents", &["hwpx"]),
        "pdf" => dialog.add_filter("PDF documents", &["pdf"]),
        "docx" => dialog.add_filter("Word documents", &["docx"]),
        "jpg" | "jpeg" => dialog.add_filter("JPEG images", &["jpg", "jpeg"]),
        _ => dialog.add_filter("HWP documents", &["hwp"]),
    }
}

fn validate_safe_path(app: &AppHandle, path: &Path) -> Result<PathBuf, String> {
    // 1. Null byte check
    if path.to_string_lossy().contains('\0') {
        return Err("Path contains invalid characters (null byte)".to_string());
    }

    // 2. Normalize and check for path traversal
    let canonical_path = if path.exists() {
        fs::canonicalize(path).map_err(|err| format!("Path normalization failed: {err}"))?
    } else if let Some(parent) = path.parent() {
        if parent.exists() {
            let canonical_parent = fs::canonicalize(parent).map_err(|err| format!("Parent path normalization failed: {err}"))?;
            let file_name = path.file_name().ok_or_else(|| "Invalid file name".to_string())?;
            canonical_parent.join(file_name)
        } else {
            return Err("Parent directory does not exist".to_string());
        }
    } else {
        return Err("Invalid path structure".to_string());
    };

    // 3. Sandbox boundary verification (HOME & App Data Directories)
    let home = app.path().home_dir().map_err(|err| format!("Failed to get home directory: {err}"))?;
    let canonical_home = fs::canonicalize(&home).unwrap_or(home);

    let app_data = app.path().app_data_dir().unwrap_or_default();
    let canonical_app_data = if app_data.exists() {
        fs::canonicalize(&app_data).unwrap_or(app_data)
    } else {
        app_data
    };

    let in_home = canonical_path.starts_with(&canonical_home);
    let in_app_data = !canonical_app_data.as_os_str().is_empty() && canonical_path.starts_with(&canonical_app_data);

    if !in_home && !in_app_data {
        return Err("Security Violation: Path is outside the allowed sandbox".to_string());
    }

    Ok(canonical_path)
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

fn path_has_supported_document_extension(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "hwp" | "hwpx"
    )
}

fn document_path_from_os_arg(arg: &OsStr, cwd: Option<&Path>) -> Option<PathBuf> {
    if let Some(raw) = arg.to_str() {
        if let Ok(url) = tauri::Url::parse(raw) {
            if let Ok(path) = url.to_file_path() {
                if path_has_supported_document_extension(&path) {
                    return Some(path);
                }
                return None;
            }
        }
    }

    let path = PathBuf::from(arg);
    let resolved = if path.is_absolute() {
        path
    } else if let Some(base) = cwd {
        base.join(path)
    } else {
        path
    };

    if !path_has_supported_document_extension(&resolved) {
        return None;
    }

    Some(resolved)
}

fn resolve_document_path(arg: &OsStr, cwd: Option<&Path>) -> Option<PathBuf> {
    document_path_from_os_arg(arg, cwd)
}

fn collect_startup_files_from_args<I, S>(args: I, cwd: Option<&Path>) -> Vec<OpenDocumentResult>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    args.into_iter()
        .filter_map(|arg| {
            let path = resolve_document_path(arg.as_ref(), cwd)?;
            open_path(&path).ok()
        })
        .collect()
}

fn collect_startup_files() -> Vec<OpenDocumentResult> {
    collect_startup_files_from_args(std::env::args_os().skip(1), None)
}

fn open_supported_paths(app: &AppHandle, paths: &[PathBuf]) -> Vec<OpenDocumentResult> {
    supported_document_paths(paths)
        .into_iter()
        .filter_map(|path| {
            let format = format_from_path(&path);
            if let Err(err) = remember_recent_document(app, &path, &format) {
                eprintln!("failed to remember dropped document: {err}");
            }
            match open_path(&path) {
                Ok(result) => Some(result),
                Err(err) => {
                    eprintln!("failed to open dropped document {}: {err}", path.display());
                    None
                }
            }
        })
        .collect()
}

fn supported_document_paths(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths
        .iter()
        .filter(|path| path_has_supported_document_extension(path))
        .cloned()
        .collect()
}

fn attach_document_drop_handler(app: &AppHandle, window: &WebviewWindow) {
    let app = app.clone();
    let label = window.label().to_string();
    window.on_window_event(move |event| {
        let WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. }) = event else {
            return;
        };

        let documents = open_supported_paths(&app, paths);
        if documents.is_empty() {
            return;
        }

        let _ = app.emit_to(label.as_str(), "rhwp://open-files", documents);
    });
}

fn next_document_window_label(
    app: &AppHandle,
    pending: &HashMap<String, Vec<OpenDocumentResult>>,
) -> String {
    let mut index = 2usize;
    loop {
        let label = format!("document-{index}");
        if app.get_webview_window(&label).is_none() && !pending.contains_key(&label) {
            return label;
        }
        index += 1;
    }
}

fn build_document_window(app: &AppHandle, label: &str) -> Result<(), String> {
    let window = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title("Geulbit X")
        .inner_size(1440.0, 980.0)
        .min_inner_size(960.0, 720.0)
        .resizable(true)
        .build()
        .map_err(|err| err.to_string())?;
    attach_document_drop_handler(app, &window);
    Ok(())
}

fn queue_open_files(
    app: &AppHandle,
    startup: &StartupFiles,
    startup_files: Vec<OpenDocumentResult>,
    reuse_main_window: bool,
) -> Result<(), String> {
    if startup_files.is_empty() {
        return Ok(());
    }

    let mut per_window = startup
        .per_window
        .lock()
        .map_err(|_| "startup files map mutex".to_string())?;

    let mut files = startup_files.into_iter();
    if reuse_main_window {
        if let Some(file) = files.next() {
            per_window.insert("main".to_string(), vec![file]);
        }
    }

    for file in files {
        let label = next_document_window_label(app, &per_window);
        per_window.insert(label.clone(), vec![file]);
        build_document_window(app, &label)?;
    }

    Ok(())
}

fn prepare_startup_windows(app: &AppHandle, startup: &StartupFiles) -> Result<(), String> {
    let startup_files = {
        let mut guard = startup
            .pending
            .lock()
            .map_err(|_| "startup files mutex".to_string())?;
        std::mem::take(&mut *guard)
    };

    queue_open_files(app, startup, startup_files, true)
}

fn pending_mime_types(is_hwp_default: bool, is_hwpx_default: bool) -> Vec<String> {
    let mut pending = Vec::new();
    if !is_hwp_default {
        pending.push(HWP_MIME.to_string());
    }
    if !is_hwpx_default {
        pending.push(HWPX_MIME.to_string());
    }
    pending
}

fn unsupported_file_association_status() -> FileAssociationStatus {
    FileAssociationStatus {
        supported: false,
        is_default: false,
        message: "이 플랫폼에서는 파일 연결 상태를 확인할 수 없습니다.".to_string(),
        platform: "unsupported".to_string(),
        action_mode: "none".to_string(),
        default_app_hwp: None,
        default_app_hwpx: None,
        pending_mime_types: vec![HWP_MIME.to_string(), HWPX_MIME.to_string()],
    }
}

fn load_file_association_status() -> Result<FileAssociationStatus, String> {
    #[cfg(target_os = "linux")]
    {
        let default_hwp = query_default_app(HWP_MIME)?;
        let default_hwpx = query_default_app(HWPX_MIME)?;
        let default_hwpx_legacy = query_default_app(HWPX_LEGACY_MIME)?;
        let is_hwp_default = default_hwp.as_deref() == Some("rhwp.desktop");
        let is_hwpx_default = default_hwpx.as_deref() == Some("rhwp.desktop")
            || default_hwpx_legacy.as_deref() == Some("rhwp.desktop");
        let pending = pending_mime_types(is_hwp_default, is_hwpx_default);
        let is_default = pending.is_empty();

        return Ok(FileAssociationStatus {
            supported: true,
            is_default,
            message: if is_default {
                "Geulbit X가 이미 HWP/HWPX 파일의 기본 앱으로 설정되어 있습니다.".to_string()
            } else {
                "HWP/HWPX 파일을 더블클릭하면 Geulbit X로 열리도록 기본 앱으로 설정하세요.".to_string()
            },
            platform: "linux".to_string(),
            action_mode: "set-default".to_string(),
            default_app_hwp: default_hwp,
            default_app_hwpx: default_hwpx.or(default_hwpx_legacy),
            pending_mime_types: pending,
        });
    }

    #[cfg(target_os = "windows")]
    {
        let (is_hwp_default, default_hwp) = query_windows_default_handler(".hwp")?;
        let (is_hwpx_default, default_hwpx) = query_windows_default_handler(".hwpx")?;
        let pending = pending_mime_types(is_hwp_default, is_hwpx_default);
        let is_default = pending.is_empty();

        return Ok(FileAssociationStatus {
            supported: true,
            is_default,
            message: if is_default {
                "Geulbit X가 이미 HWP/HWPX 파일의 기본 앱으로 설정되어 있습니다.".to_string()
            } else {
                "Windows에서는 기본 앱을 사용자가 확인해야 합니다. 기본 앱 설정을 열어 .hwp와 .hwpx에 Geulbit X를 선택하세요.".to_string()
            },
            platform: "windows".to_string(),
            action_mode: "open-settings".to_string(),
            default_app_hwp: default_hwp,
            default_app_hwpx: default_hwpx,
            pending_mime_types: pending,
        });
    }

    #[allow(unreachable_code)]
    Ok(unsupported_file_association_status())
}

#[cfg(target_os = "linux")]
fn query_default_app(mime_type: &str) -> Result<Option<String>, String> {
    let output = std::process::Command::new("xdg-mime")
        .args(["query", "default", mime_type])
        .output()
        .map_err(|err| err.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() {
        if stdout.is_empty() {
            return Ok(None);
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(stderr);
    }

    if stdout.is_empty() {
        Ok(None)
    } else {
        Ok(Some(stdout))
    }
}

#[cfg(target_os = "windows")]
fn query_registry_default_string(key: &RegKey) -> Option<String> {
    key.get_value::<String, _>("")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(target_os = "windows")]
fn query_windows_prog_id(extension: &str) -> Result<Option<String>, String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let user_choice_path = format!(
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\{extension}\UserChoice"
    );

    if let Ok(key) = hkcu.open_subkey(&user_choice_path) {
        if let Ok(prog_id) = key.get_value::<String, _>("ProgId") {
            let trimmed = prog_id.trim();
            if !trimmed.is_empty() {
                return Ok(Some(trimmed.to_string()));
            }
        }
    }

    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    let Ok(key) = hkcr.open_subkey(extension) else {
        return Ok(None);
    };
    Ok(query_registry_default_string(&key))
}

#[cfg(target_os = "windows")]
fn resolve_windows_open_command(prog_id: &str) -> Result<Option<String>, String> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    let key_path = format!(r"{prog_id}\shell\open\command");
    let Ok(key) = hkcr.open_subkey(&key_path) else {
        return Ok(None);
    };
    Ok(query_registry_default_string(&key))
}

#[cfg(target_os = "windows")]
fn parse_windows_command_executable(command: &str) -> Option<PathBuf> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return None;
    }

    let executable = if let Some(rest) = trimmed.strip_prefix('"') {
        rest.split('"').next()?
    } else {
        trimmed.split_whitespace().next()?
    };

    let path = PathBuf::from(executable);
    if path.as_os_str().is_empty() {
        None
    } else {
        Some(path)
    }
}

#[cfg(target_os = "windows")]
fn current_executable_path() -> Option<PathBuf> {
    let path = std::env::current_exe().ok()?;
    Some(fs::canonicalize(&path).unwrap_or(path))
}

#[cfg(target_os = "windows")]
fn is_current_executable(path: &Path) -> bool {
    let Some(current) = current_executable_path() else {
        return false;
    };
    let candidate = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    candidate
        .to_string_lossy()
        .eq_ignore_ascii_case(&current.to_string_lossy())
}

#[cfg(target_os = "windows")]
fn query_windows_default_handler(extension: &str) -> Result<(bool, Option<String>), String> {
    let Some(prog_id) = query_windows_prog_id(extension)? else {
        return Ok((false, None));
    };
    let Some(command) = resolve_windows_open_command(&prog_id)? else {
        return Ok((false, Some(prog_id)));
    };
    let Some(executable) = parse_windows_command_executable(&command) else {
        return Ok((false, Some(command)));
    };

    let resolved = executable.to_string_lossy().to_string();
    Ok((is_current_executable(&executable), Some(resolved)))
}

#[tauri::command]
fn open_document(app: AppHandle) -> Result<Option<OpenDocumentResult>, String> {
    let picked = rfd::FileDialog::new()
        .add_filter("HWP documents", &["hwp", "hwpx"])
        .pick_file();

    let Some(path) = picked else {
        return Ok(None);
    };

    let path = validate_safe_path(&app, &path)?;
    let format = format_from_path(&path);
    remember_recent_document(&app, &path, &format)?;
    open_path(&path).map(Some)
}

#[tauri::command]
fn open_document_at_path(app: AppHandle, path: String) -> Result<OpenDocumentResult, String> {
    let raw_path = PathBuf::from(path);
    let path = validate_safe_path(&app, &raw_path)?;
    if !path_has_supported_document_extension(&path) {
        return Err("unsupported document path".to_string());
    }

    let format = format_from_path(&path);
    remember_recent_document(&app, &path, &format)?;
    open_path(&path)
}

#[tauri::command]
fn consume_startup_files(
    window: Window,
    startup: State<StartupFiles>,
) -> Result<Vec<OpenDocumentResult>, String> {
    let mut guard = startup
        .per_window
        .lock()
        .map_err(|_| "startup files map mutex".to_string())?;
    Ok(guard.remove(window.label()).unwrap_or_default())
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

    let selected_path = normalize_path_extension(
        selected_path,
        document_extension_for_format(&request.format),
    );
    let selected_path = validate_safe_path(&app, &selected_path)?;

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
fn save_binary_file(app: AppHandle, request: SaveBinaryRequest) -> Result<Option<SaveBinaryResult>, String> {
    let target_path = request.file_path.clone().map(PathBuf::from);

    let selected_path = match target_path {
        Some(path) => path,
        None => {
            let suggested_name = normalize_binary_name(&request.suggested_name, &request.format);
            let picked = save_dialog_for_binary_format(
                rfd::FileDialog::new().set_file_name(&suggested_name),
                &request.format,
            )
            .save_file();

            let Some(path) = picked else {
                return Ok(None);
            };
            path
        }
    };

    let selected_path =
        normalize_path_extension(selected_path, binary_extension_for_format(&request.format));
    let selected_path = validate_safe_path(&app, &selected_path)?;

    if let Some(parent) = selected_path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }

    fs::write(&selected_path, &request.bytes).map_err(|err| err.to_string())?;

    Ok(Some(SaveBinaryResult {
        file_name: selected_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("export")
            .to_string(),
        file_path: Some(selected_path.to_string_lossy().to_string()),
        format: request.format,
    }))
}

#[tauri::command]
fn pick_export_directory() -> Result<Option<String>, String> {
    Ok(rfd::FileDialog::new()
        .pick_folder()
        .map(|path| path.to_string_lossy().to_string()))
}

#[tauri::command]
fn export_pdf_from_svgs(svgs: Vec<String>) -> Result<Vec<u8>, String> {
    rhwp::renderer::pdf::svgs_to_pdf(&svgs).map_err(|err| err.to_string())
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
        for mime_type in [HWP_MIME, HWPX_MIME, HWPX_LEGACY_MIME] {
            let status = std::process::Command::new("xdg-mime")
                .args(["default", "rhwp.desktop", mime_type])
                .status()
                .map_err(|err| err.to_string())?;
            if !status.success() {
                return Err(format!("xdg-mime failed for {mime_type}"));
            }
        }

        return load_file_association_status();
    }

    #[cfg(target_os = "windows")]
    {
        let status = std::process::Command::new("cmd")
            .args(["/C", "start", "", "ms-settings:defaultapps"])
            .status()
            .map_err(|err| err.to_string())?;
        if !status.success() {
            return Err("failed to open Windows Default Apps settings".to_string());
        }

        let mut association_status = load_file_association_status()?;
        if !association_status.is_default {
            association_status.message =
                "Windows Default Apps Settings opened. Choose Geulbit X for .hwp and .hwpx.".to_string();
        }
        return Ok(association_status);
    }

    #[allow(unreachable_code)]
    Ok(unsupported_file_association_status())
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

    fs::write(recovery_data_path(&dir, &snapshot_id), request.bytes)
        .map_err(|err| err.to_string())?;
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

    #[cfg(target_os = "windows")]
    {
        let target = PathBuf::from(path);
        let selection = format!("/select,\"{}\"", target.to_string_lossy());
        std::process::Command::new("explorer.exe")
            .arg(selection)
            .spawn()
            .map_err(|err| err.to_string())?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    let _ = &path;

    #[allow(unreachable_code)]
    Ok(())
}

#[tauri::command]
fn force_close_window(window: tauri::Window) {
    let _ = window.destroy();
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            let cwd_path = if cwd.trim().is_empty() {
                None
            } else {
                Some(Path::new(cwd.as_str()))
            };
            let startup = app.state::<StartupFiles>();
            let files = collect_startup_files_from_args(args, cwd_path);
            if let Err(err) = queue_open_files(app, startup.inner(), files, false) {
                eprintln!("failed to queue startup files from secondary instance: {err}");
            }
        }))
        .manage(StartupFiles {
            pending: Mutex::new(collect_startup_files()),
            per_window: Mutex::new(HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![
            open_document,
            open_document_at_path,
            consume_startup_files,
            save_document,
            save_binary_file,
            pick_export_directory,
            export_pdf_from_svgs,
            get_recent_documents,
            get_file_association_status,
            set_default_file_association,
            list_recovery_snapshots,
            read_recovery_snapshot,
            write_recovery_snapshot,
            delete_recovery_snapshot,
            reveal_in_folder,
            force_close_window
        ])
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                attach_document_drop_handler(app.handle(), &window);
            }
            let startup = app.state::<StartupFiles>();
            let handle = app.handle().clone();
            prepare_startup_windows(&handle, startup.inner()).map_err(std::io::Error::other)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running rhwp-desktop");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_path_from_os_arg_accepts_supported_extensions() {
        assert!(document_path_from_os_arg(OsStr::new("sample.hwp"), None).is_some());
        assert!(document_path_from_os_arg(OsStr::new("sample.HWPX"), None).is_some());
        assert!(document_path_from_os_arg(OsStr::new("sample.pdf"), None).is_none());
    }

    #[test]
    fn document_path_from_os_arg_resolves_relative_paths_against_cwd() {
        let cwd = std::env::temp_dir().join("rhwp-startup-arg");
        let expected = cwd.join("docs").join("sample.hwp");

        assert_eq!(
            document_path_from_os_arg(OsStr::new("docs/sample.hwp"), Some(cwd.as_path())),
            Some(expected)
        );
    }

    #[test]
    fn document_path_from_os_arg_accepts_file_urls() {
        let path = std::env::temp_dir().join("sample.hwpx");
        let url = tauri::Url::from_file_path(&path).unwrap().to_string();

        assert_eq!(
            document_path_from_os_arg(OsStr::new(url.as_str()), Some(Path::new("/ignored"))),
            Some(path)
        );
    }

    #[test]
    fn collect_startup_files_from_args_skips_unsupported_inputs() {
        let dir = std::env::temp_dir().join(format!("rhwp-startup-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let supported = dir.join("opened.hwp");
        let ignored = dir.join("ignored.txt");
        fs::write(&supported, b"doc").unwrap();
        fs::write(&ignored, b"ignore").unwrap();

        let opened = collect_startup_files_from_args(
            [supported.as_os_str(), ignored.as_os_str()],
            Some(dir.as_path()),
        );

        assert_eq!(opened.len(), 1);
        assert_eq!(opened[0].file_name, "opened.hwp");

        let _ = fs::remove_file(&supported);
        let _ = fs::remove_file(&ignored);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn supported_document_paths_filters_unsupported_paths() {
        let dir = std::env::temp_dir().join(format!("rhwp-drop-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let supported = dir.join("dropped.HWPX");
        let ignored = dir.join("ignored.pdf");
        fs::write(&supported, b"doc").unwrap();
        fs::write(&ignored, b"pdf").unwrap();

        let paths = supported_document_paths(&[supported.clone(), ignored.clone()]);

        assert_eq!(paths, vec![supported.clone()]);

        let _ = fs::remove_file(&supported);
        let _ = fs::remove_file(&ignored);
        let _ = fs::remove_dir(&dir);
    }
}
