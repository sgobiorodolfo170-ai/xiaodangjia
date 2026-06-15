mod db;
mod watcher;
mod ast_parser;
mod tfidf;
mod semantic_search;
mod semantic_embedding;
mod plugin_system;

use db::{Database, Project, FileNode};
use std::sync::Mutex;
use tauri::{State, AppHandle, Manager};
use walkdir::WalkDir;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::fs;
use std::path::Path;

pub struct AppState {
    db: Mutex<Database>,
}

// Helper: validate that a path is within the project root
fn validate_path_in_project(path: &str, project_root: &str) -> Result<(), String> {
    let canonical = Path::new(path)
        .canonicalize()
        .map_err(|_| format!("Invalid path: {}", path))?;
    let root = Path::new(project_root)
        .canonicalize()
        .map_err(|_| format!("Invalid project root: {}", project_root))?;
    if !canonical.starts_with(&root) {
        return Err(format!("Path is outside project root: {}", path));
    }
    Ok(())
}

// Project commands
#[tauri::command]
fn create_project(name: String, root_path: String, state: State<AppState>) -> Result<Project, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.create_project(&name, &root_path).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_projects(state: State<AppState>) -> Result<Vec<Project>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_projects().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_project(id: String, state: State<AppState>) -> Result<Option<Project>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_project(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_project(id: String, state: State<AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_project(&id).map_err(|e| e.to_string())
}

// File scan command
#[tauri::command]
fn scan_directory(project_id: String, path: String, state: State<AppState>) -> Result<Vec<FileNode>, String> {
    let root_path = Path::new(&path);
    if !root_path.exists() {
        return Err("Path does not exist".to_string());
    }

    let mut nodes = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    for entry in WalkDir::new(&path).into_iter().filter_map(|e| match e {
        Ok(e) => Some(e),
        Err(err) => {
            errors.push(format!("Error accessing {}: {}", err.path().unwrap_or_else(|| Path::new("?")).display(), err));
            None
        }
    }) {
        let entry_path = entry.path();
        let metadata = entry_path.metadata().ok();

        let is_dir = entry_path.is_dir();
        let name = entry_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let extension = if is_dir {
            String::new()
        } else {
            entry_path.extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default()
        };

        let (created_at, modified_at, size) = match metadata {
            Some(m) => {
                let created = m.created().ok()
                    .map(|t| DateTime::<Utc>::from(t).to_rfc3339());
                let modified = m.modified().ok()
                    .map(|t| DateTime::<Utc>::from(t).to_rfc3339());
                let size = m.len() as i64;
                (created, modified, size)
            }
            None => (None, None, 0),
        };

        // Compute parent_id correctly by matching parent path
        let parent_id = entry_path.parent().and_then(|parent| {
            let parent_str = parent.to_string_lossy().to_string();
            if parent_str == path {
                None
            } else {
                nodes.iter().find(|n: &&FileNode| n.path == parent_str).map(|n| n.id.clone())
            }
        });

        let position_index = nodes.len() as f64;
        let x = (position_index % 5.0) * 250.0;
        let y = (position_index / 5.0) * 150.0;

        // Generate tags
        let mut tags = vec![];
        let ext_lower = extension.to_lowercase();
        let name_lower = name.to_lowercase();

        match ext_lower.as_str() {
            "js" | "jsx" | "ts" | "tsx" => tags.push("JavaScript".to_string()),
            "py" => tags.push("Python".to_string()),
            "rs" => tags.push("Rust".to_string()),
            "go" => tags.push("Go".to_string()),
            "java" => tags.push("Java".to_string()),
            "json" => tags.push("Config".to_string()),
            "md" => tags.push("文档".to_string()),
            "png" | "jpg" | "jpeg" | "gif" | "svg" => tags.push("图片".to_string()),
            "mp4" | "webm" | "mkv" => tags.push("视频".to_string()),
            "mp3" | "wav" | "ogg" => tags.push("音频".to_string()),
            "zip" | "rar" | "7z" => tags.push("压缩包".to_string()),
            _ => {}
        }

        if name_lower.contains("test") || name_lower.contains("spec") {
            tags.push("测试".to_string());
        }
        if name_lower.contains("config") {
            tags.push("配置".to_string());
        }
        if name_lower.starts_with('.') {
            tags.push("隐藏文件".to_string());
        }
        if size > 10 * 1024 * 1024 {
            tags.push("大文件".to_string());
        }
        if is_dir {
            tags.push("目录".to_string());
        }

        let node = FileNode {
            id: Uuid::new_v4().to_string(),
            project_id: project_id.clone(),
            path: entry_path.to_string_lossy().to_string(),
            name,
            extension,
            size,
            created_at,
            modified_at,
            tags,
            parent_id,
            position_x: x,
            position_y: y,
            is_collapsed: false,
            is_directory: is_dir,
        };

        nodes.push(node);
    }

    // Only persist to DB after successful scan
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_file_nodes_by_project(&project_id).map_err(|e| e.to_string())?;
    for node in &nodes {
        let _ = db.insert_file_node(node);
    }

    if !errors.is_empty() {
        log::warn!("Scan completed with {} errors: {:?}", errors.len(), errors);
    }

    Ok(nodes)
}

// File content commands
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileContent {
    path: String,
    content: String,
    encoding: String,
    size: u64,
}

#[tauri::command]
fn read_file_content(path: String) -> Result<FileContent, String> {
    let path_obj = Path::new(&path);
    if !path_obj.exists() {
        return Err("File does not exist".to_string());
    }

    if path_obj.is_dir() {
        return Err("Cannot read directory".to_string());
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let metadata = fs::metadata(&path)
        .map_err(|e| format!("Failed to get metadata: {}", e))?;

    Ok(FileContent {
        path,
        content,
        encoding: "utf-8".to_string(),
        size: metadata.len(),
    })
}

#[tauri::command]
fn write_file_content(path: String, content: String) -> Result<(), String> {
    fs::write(&path, content)
        .map_err(|e| format!("Failed to write file: {}", e))
}

#[tauri::command]
fn delete_file(project_id: String, path: String, state: State<AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let root = db.get_project_root(&project_id)
        .map_err(|e| e.to_string())?
        .ok_or("Project not found")?;
    validate_path_in_project(&path, &root)?;

    let path_obj = Path::new(&path);
    if path_obj.is_dir() {
        fs::remove_dir_all(&path)
            .map_err(|e| format!("Failed to delete directory: {}", e))
    } else {
        fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete file: {}", e))
    }
}

#[tauri::command]
fn rename_file(project_id: String, old_path: String, new_path: String, state: State<AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let root = db.get_project_root(&project_id)
        .map_err(|e| e.to_string())?
        .ok_or("Project not found")?;
    validate_path_in_project(&old_path, &root)?;
    validate_path_in_project(&new_path, &root)?;

    fs::rename(&old_path, &new_path)
        .map_err(|e| format!("Failed to rename file: {}", e))
}

// Node position command
#[tauri::command]
fn update_node_position(id: String, x: f64, y: f64, state: State<AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.update_node_position(&id, x, y).map_err(|e| e.to_string())
}

// AI Agent commands
#[tauri::command]
fn analyze_file_relations(project_id: String, state: State<AppState>) -> Result<Vec<db::FileRelation>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let nodes = db.get_file_nodes_by_project(&project_id).map_err(|e| e.to_string())?;

    let relations = watcher::analyze_relations(&nodes);
    log::info!("Analyzed {} relations for project {}", relations.len(), project_id);

    Ok(relations)
}

#[tauri::command]
fn generate_tags(project_id: String, file_id: String, state: State<AppState>) -> Result<Vec<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let nodes = db.get_file_nodes_by_project(&project_id).map_err(|e| e.to_string())?;

    let node = nodes.iter().find(|n| n.id == file_id)
        .ok_or("File not found")?;

    let tags = watcher::generate_file_tags(node);
    log::info!("Generated {} tags for file {}", tags.len(), file_id);

    Ok(tags)
}

#[tauri::command]
fn search_files(project_id: String, query: String, state: State<AppState>) -> Result<Vec<FileNode>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let nodes = db.get_file_nodes_by_project(&project_id).map_err(|e| e.to_string())?;

    let results = watcher::search_files(&nodes, &query);
    log::info!("Search '{}' found {} results", query, results.len());

    Ok(results)
}

#[tauri::command]
fn find_similar_files(project_id: String, file_id: String, state: State<AppState>) -> Result<Vec<FileNode>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let nodes = db.get_file_nodes_by_project(&project_id).map_err(|e| e.to_string())?;

    let target = nodes.iter().find(|n| n.id == file_id)
        .ok_or("File not found")?;

    let similar = watcher::find_similar_files(&nodes, target);
    log::info!("Found {} similar files for {}", similar.len(), file_id);

    Ok(similar)
}

// File watcher command
#[tauri::command]
fn start_file_watcher(project_id: String, path: String, app_handle: AppHandle) -> Result<(), String> {
    watcher::start_file_watcher(app_handle, project_id, path)
}

// Dialog command - delegates to the Tauri dialog plugin
#[tauri::command]
async fn open_directory_dialog(app_handle: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = std::sync::mpsc::channel();

    app_handle.dialog()
        .file()
        .pick_folder(move |path| {
            let _ = tx.send(path.map(|p| p.to_string()));
        });

    rx.recv().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()
                .expect("Failed to get app data directory");

            let db = Database::new(app_data_dir)
                .expect("Failed to initialize database");

            app.manage(AppState {
                db: Mutex::new(db),
            });

            log::info!("小当家应用启动成功");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_project,
            list_projects,
            get_project,
            delete_project,
            scan_directory,
            read_file_content,
            write_file_content,
            delete_file,
            rename_file,
            update_node_position,
            analyze_file_relations,
            generate_tags,
            search_files,
            find_similar_files,
            start_file_watcher,
            open_directory_dialog,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
