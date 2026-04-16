use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

#[derive(Default)]
struct StartupFiles {
    files: Mutex<Vec<OpenDocumentResult>>,
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
    match path.extension().and_then(|ext| ext.to_str()).unwrap_or_default().to_ascii_lowercase().as_str() {
        "hwpx" => "hwpx".to_string(),
        _ => "hwp".to_string(),
    }
}

fn recents_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_dir = app.path().app_data_dir().map_err(|err| err.to_string())?;
    fs::create_dir_all(&app_dir).map_err(|err| err.to_string())?;
    Ok(app_dir.join("recent-documents.json"))
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
fn save_document(app: AppHandle, request: SaveDocumentRequest) -> Result<Option<SaveDocumentResult>, String> {
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
fn reveal_in_folder(path: String) -> Result<(), String> {
    let target = PathBuf::from(path);
    let parent = target.parent().unwrap_or(target.as_path());

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|err| err.to_string())?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Ok(())
}

fn emit_startup_files(window: &WebviewWindow, startup: &State<StartupFiles>) {
    let payload = {
        let mut guard = startup.files.lock().expect("startup files mutex");
        if guard.is_empty() {
            return;
        }
        std::mem::take(&mut *guard)
    };

    let _ = window.emit("rhwp://open-files", payload);
}

fn main() {
    tauri::Builder::default()
        .manage(StartupFiles {
            files: Mutex::new(collect_startup_files()),
        })
        .invoke_handler(tauri::generate_handler![
            open_document,
            save_document,
            get_recent_documents,
            reveal_in_folder
        ])
        .setup(|_app| Ok(()))
        .on_page_load(|window, _| {
            emit_startup_files(window, &window.state::<StartupFiles>());
        })
        .run(tauri::generate_context!())
        .expect("error while running rhwp-desktop");
}
