mod db;
mod watcher;
mod analysis;
mod ast_parser;
mod tfidf;
mod semantic_search;
mod semantic_embedding;

use db::{Database, Project, FileNode};
use std::sync::Mutex;
use tauri::{State, AppHandle, Manager};
use walkdir::WalkDir;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::fs;
use std::path::Path;
use std::collections::HashMap;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

struct CachedTfidf {
    index: tfidf::TfidfIndex,
    file_ids: Vec<String>,
    file_names: HashMap<String, String>,
}

pub struct AppState {
    db: Mutex<Database>,
    watcher_handles: Mutex<HashMap<String, std::thread::JoinHandle<()>>>,
    watcher_stop_flags: Mutex<HashMap<String, Arc<AtomicBool>>>,
    project_roots: RwLock<HashMap<String, String>>,
    tfidf_cache: Mutex<HashMap<String, CachedTfidf>>,
}

/// Get project root path with in-memory cache (avoids repeated DB queries).
/// On cache miss, falls back to DB and populates the cache.
fn get_cached_project_root(project_id: &str, state: &AppState) -> Result<String, String> {
    {
        let roots = state.project_roots.read().map_err(|e| e.to_string())?;
        if let Some(root) = roots.get(project_id) {
            return Ok(root.clone());
        }
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let root = db.get_project_root(project_id).map_err(|e| e.to_string())?
        .ok_or("Project not found")?;
    drop(db);
    {
        let mut roots = state.project_roots.write().map_err(|e| e.to_string())?;
        roots.insert(project_id.to_string(), root.clone());
    }
    Ok(root)
}

/// Invalidate cached project root (call after delete_project).
fn invalidate_project_root(project_id: &str, state: &AppState) {
    if let Ok(mut roots) = state.project_roots.write() {
        roots.remove(project_id);
    }
}

// Helper: validate that a path is within the project root.
// Handles non-existent paths by canonicalizing the existing parent instead.
fn validate_path_in_project(path: &str, project_root: &str) -> Result<(), String> {
    let canonical = Path::new(path).canonicalize().or_else(|_| {
        // File may not exist yet (e.g. create_directory, write_file_content).
        // Fall back to canonicalizing the parent directory.
        Path::new(path)
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.canonicalize())
            .unwrap_or(Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Cannot resolve path: {}", path),
            )))
    }).map_err(|e| format!("Invalid path '{}': {}", path, e))?;

    let root = Path::new(project_root)
        .canonicalize()
        .map_err(|_| format!("Invalid project root: {}", project_root))?;
    if !canonical.starts_with(&root) {
        return Err(format!("Path '{}' is outside project root '{}'", path, project_root));
    }
    Ok(())
}

fn fill_children_and_relations(nodes: &mut Vec<FileNode>, db: &Database, project_id: &str) {
    let children_map: std::collections::HashMap<String, Vec<String>> = {
        let mut map = std::collections::HashMap::new();
        for node in nodes.iter() {
            if let Some(pid) = &node.parent_id {
                map.entry(pid.clone()).or_insert_with(Vec::new).push(node.id.clone());
            }
        }
        map
    };

    let relations_map: std::collections::HashMap<String, Vec<String>> = {
        let mut map = std::collections::HashMap::new();
        if let Ok(relations) = db.get_relations_by_project(project_id) {
            for rel in relations {
                map.entry(rel.source_id.clone()).or_insert_with(Vec::new).push(rel.target_id.clone());
                map.entry(rel.target_id.clone()).or_insert_with(Vec::new).push(rel.source_id.clone());
            }
        }
        map
    };

    for node in nodes.iter_mut() {
        node.children = children_map.get(&node.id).cloned().unwrap_or_default();
        node.related_files = relations_map.get(&node.id).cloned().unwrap_or_default();
    }
}

// Project commands
#[tauri::command]
fn create_project(name: String, root_path: String, state: State<AppState>) -> Result<Project, String> {
    if !Path::new(&root_path).exists() {
        return Err(format!("Root path does not exist: {}", root_path));
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let project = db.create_project(&name, &root_path).map_err(|e| e.to_string())?;
    drop(db);
    {
        let mut roots = state.project_roots.write().map_err(|e| e.to_string())?;
        roots.insert(project.id.clone(), root_path);
    }
    Ok(project)
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
    db.delete_project(&id).map_err(|e| e.to_string())?;
    invalidate_project_root(&id, &state);
    Ok(())
}

// File scan command
#[tauri::command]
async fn scan_directory(
    project_id: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<Vec<FileNode>, String> {
    let root_path = Path::new(&path);
    if !root_path.exists() {
        return Err("Path does not exist".to_string());
    }

    // Heavy filesystem traversal runs on a blocking thread so the Tauri main
    // thread (and the UI) stays responsive. No DB lock is held here.
    let path_clone = path.clone();
    let project_id_clone = project_id.clone();
    let nodes_result = tauri::async_runtime::spawn_blocking(move || -> Result<(Vec<FileNode>, Vec<String>), String> {
        scan_directory_blocking(&project_id_clone, &path_clone)
    })
    .await
    .map_err(|e| format!("Scan task failed: {}", e))?;

    let (mut nodes, errors) = nodes_result?;

    // Persist to DB atomically — replace (delete + batch insert) in one transaction.
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.replace_file_nodes(&project_id, &nodes)
        .map_err(|e| format!("Failed to persist scan: {}", e))?;
    fill_children_and_relations(&mut nodes, &db, &project_id);

    if !errors.is_empty() {
        log::warn!("Scan completed with {} errors: {:?}", errors.len(), errors);
    }

    Ok(nodes)
}

/// Directories that are typically huge and not useful to show on the canvas.
/// They are pruned during traversal to keep scan time short.
const PRUNED_DIR_NAMES: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "dist",
    "build",
    "__pycache__",
    ".venv",
    "venv",
    ".next",
    ".nuxt",
    ".cache",
    ".idea",
    ".vscode",
    "coverage",
    ".gradle",
    ".terraform",
];

/// Maximum directory depth to traverse. Root counts as depth 0.
const MAX_SCAN_DEPTH: usize = 5;

/// Blocking filesystem traversal. Builds FileNode list using an O(1) path→id map
/// for parent_id resolution (replaces the previous O(N²) linear scan).
fn scan_directory_blocking(project_id: &str, path: &str) -> Result<(Vec<FileNode>, Vec<String>), String> {
    let mut nodes: Vec<FileNode> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    // path -> node id, for O(1) parent lookup
    let mut path_to_id: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    for entry in WalkDir::new(path)
        .max_depth(MAX_SCAN_DEPTH)
        .into_iter()
        .filter_entry(|e| {
            // Prune large/irrelevant directories before descending into them.
            if e.file_type().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    if PRUNED_DIR_NAMES.contains(&name) {
                        return false;
                    }
                }
            }
            true
        })
        .filter_map(|e| match e {
            Ok(e) => Some(e),
            Err(err) => {
                errors.push(format!(
                    "Error accessing {}: {}",
                    err.path().unwrap_or_else(|| Path::new("?")).display(),
                    err
                ));
                None
            }
        })
    {
        let entry_path = entry.path();
        let metadata = entry_path.metadata().ok();

        let is_dir = entry_path.is_dir();
        let name = entry_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let extension = if is_dir {
            String::new()
        } else {
            entry_path
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default()
        };

        let (created_at, modified_at, size) = match metadata {
            Some(m) => {
                let created = m
                    .created()
                    .ok()
                    .map(|t| DateTime::<Utc>::from(t).to_rfc3339());
                let modified = m
                    .modified()
                    .ok()
                    .map(|t| DateTime::<Utc>::from(t).to_rfc3339());
                let size = m.len() as i64;
                (created, modified, size)
            }
            None => (None, None, 0),
        };

        // O(1) parent_id lookup via HashMap (previously O(N) linear search).
        let parent_id = entry_path.parent().and_then(|parent| {
            let parent_str = parent.to_string_lossy().to_string();
            if parent_str == path {
                None
            } else {
                path_to_id.get(&parent_str).cloned()
            }
        });

        let position_index = nodes.len() as f64;
        let x = (position_index % 5.0) * 250.0;
        let y = (position_index / 5.0) * 150.0;

        // Generate tags via shared analysis module
        let tags = analysis::tags::generate_file_tags(&FileNode {
            id: String::new(), // placeholder ? tag generation doesn't use id
            project_id: project_id.to_string(),
            path: entry_path.to_string_lossy().to_string(),
            name: name.clone(),
            extension: extension.clone(),
            size,
            created_at: created_at.clone(),
            modified_at: modified_at.clone(),
            tags: vec![],
            parent_id: None,
            position_x: 0.0,
            position_y: 0.0,
            is_collapsed: false,
            is_directory: is_dir,
            children: vec![],
            related_files: vec![],
        });

        let id = Uuid::new_v4().to_string();
        // Index this node before pushing so children can find it as a parent.
        path_to_id.insert(entry_path.to_string_lossy().to_string(), id.clone());

        let node = FileNode {
            id,
            project_id: project_id.to_string(),
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
            children: Vec::new(),
            related_files: Vec::new(),
        };

        nodes.push(node);
    }

    Ok((nodes, errors))
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
fn read_file_content(path: String, project_id: String, state: State<AppState>) -> Result<FileContent, String> {
    let root = get_cached_project_root(&project_id, &state)?;
    validate_path_in_project(&path, &root)?;
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
fn write_file_content(path: String, content: String, file_id: Option<String>, project_id: String, state: State<AppState>) -> Result<(), String> {
    let root = get_cached_project_root(&project_id, &state)?;
    validate_path_in_project(&path, &root)?;

    // Read original content for diff calculation (before writing)
    let old_content = fs::read_to_string(&path).unwrap_or_default();

    // Write the file
    fs::write(&path, &content)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    // Save edit history after successful write (db lock acquired separately to avoid deadlock)
    if old_content != content {
        if let Some(fid) = &file_id {
            let diff = similar::TextDiff::from_lines(&old_content, &content);
            let diff_text = diff.unified_diff().to_string();
            let db = state.db.lock().map_err(|e| e.to_string())?;
            if let Err(e) = db.add_file_edit_history(fid, &path, &content, Some(&diff_text)) {
                log::warn!("Failed to save edit history: {}", e);
            }
        }
    }

    Ok(())
}

// ============= File Edit History Commands =============

#[tauri::command]
fn get_file_history(file_id: String, state: State<AppState>) -> Result<Vec<db::FileEditHistory>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_file_edit_history(&file_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn restore_file_version(version_id: String, state: State<AppState>) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    
    // Get the version to restore
    let version = db.get_file_edit_history_by_id(&version_id)
        .map_err(|e| e.to_string())?
        .ok_or("Version not found")?;
    
    // Write the content back to file
    fs::write(&version.file_path, &version.content)
        .map_err(|e| format!("Failed to restore file: {}", e))?;
    
    // Add new history entry for the restore
    let _ = db.add_file_edit_history(&version.file_id, &version.file_path, &version.content, Some("Restored from history"))
        .map_err(|e| log::warn!("Failed to save restore history: {}", e));
    
    log::info!("Restored file {} from version {}", version.file_path, version_id);
    Ok(version.content)
}

#[tauri::command]
fn delete_file_history_version(version_id: String, state: State<AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_file_edit_history(&version_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_file(project_id: String, path: String, state: State<AppState>) -> Result<(), String> {
    let root = get_cached_project_root(&project_id, &state)?;
    validate_path_in_project(&path, &root)?;

    let path_obj = Path::new(&path);
    if path_obj.is_dir() {
        fs::remove_dir_all(&path)
            .map_err(|e| format!("Failed to delete directory: {}", e))?;
    } else {
        fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete file: {}", e))?;
    }

    // P2-1: Sync DB — remove the file node after successful disk deletion
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let nodes = db.get_file_nodes_by_project(&project_id).map_err(|e| e.to_string())?;
    if nodes.iter().any(|n| n.path == path) {
        db.delete_file_node_by_path(&path).map_err(|e| e.to_string())?;
        log::info!("Removed file node from DB after delete: {}", path);
    }

    Ok(())
}

#[tauri::command]
fn rename_file(project_id: String, old_path: String, new_path: String, state: State<AppState>) -> Result<(), String> {
    let root = get_cached_project_root(&project_id, &state)?;
    validate_path_in_project(&old_path, &root)?;
    validate_path_in_project(&new_path, &root)?;

    fs::rename(&old_path, &new_path)
        .map_err(|e| format!("Failed to rename file: {}", e))?;

    // P2-1: Sync DB — update the file node path and name after successful rename
    let new_name = Path::new(&new_path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();
    let new_ext = Path::new(&new_path)
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.update_file_node_path(&old_path, &new_path, &new_name, &new_ext)
        .map_err(|e| e.to_string())?;
    log::info!("Updated file node in DB after rename: {} -> {}", old_path, new_path);

    Ok(())
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

    let relations = analysis::relations::analyze_relations(&nodes);
    log::info!("Analyzed {} relations for project {}", relations.len(), project_id);

    Ok(relations)
}

#[tauri::command]
fn generate_tags(project_id: String, file_id: String, state: State<AppState>) -> Result<Vec<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let node = db.get_file_node_by_id(&file_id).map_err(|e| e.to_string())?
        .ok_or("File not found")?;

    let tags = analysis::tags::generate_file_tags(&node);
    log::info!("Generated {} tags for file {}", tags.len(), file_id);

    Ok(tags)
}

#[tauri::command]
fn search_files(project_id: String, query: String, state: State<AppState>) -> Result<Vec<FileNode>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let nodes = db.get_file_nodes_by_project(&project_id).map_err(|e| e.to_string())?;

    let results = analysis::search::search_files(&nodes, &query);
    log::info!("Search '{}' found {} results", query, results.len());

    Ok(results)
}

#[tauri::command]
fn find_similar_files(project_id: String, file_id: String, state: State<AppState>) -> Result<Vec<FileNode>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let nodes = db.get_file_nodes_by_project(&project_id).map_err(|e| e.to_string())?;

    let target = nodes.iter().find(|n| n.id == file_id)
        .ok_or("File not found")?;

    let similar = analysis::search::find_similar_files(&nodes, target);
    log::info!("Found {} similar files for {}", similar.len(), file_id);

    Ok(similar)
}

// Semantic search command
#[tauri::command]
fn semantic_search_files(project_id: String, query: String, state: State<AppState>) -> Result<Vec<FileNode>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let nodes = db.get_file_nodes_by_project(&project_id).map_err(|e| e.to_string())?;

    let search = semantic_search::SemanticSearch::new();
    let items: Vec<semantic_search::SearchableItem> = nodes.iter().map(|n| semantic_search::SearchableItem {
        id: n.id.clone(),
        name: n.name.clone(),
        path: n.path.clone(),
        extension: n.extension.clone(),
        tags: n.tags.clone(),
    }).collect();

    let results = search.search(&items, &query);
    let result_ids: Vec<String> = results.iter().map(|r| r.item.id.clone()).collect();

    let matched: Vec<FileNode> = nodes.into_iter()
        .filter(|n| result_ids.contains(&n.id))
        .collect();

    log::info!("Semantic search '{}' found {} results", query, matched.len());
    Ok(matched)
}

// TF-IDF content similarity command
#[tauri::command]
fn find_similar_by_content(project_id: String, file_id: String, top_k: Option<usize>, state: State<AppState>) -> Result<Vec<tfidf::TfidfSimilarityResult>, String> {
    let (target, nodes) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let nodes = db.get_file_nodes_by_project(&project_id).map_err(|e| e.to_string())?;
        let target = nodes.iter().find(|n| n.id == file_id)
            .ok_or("File not found")?
            .clone();
        (target, nodes)
    };

    let target_content = fs::read_to_string(&target.path).unwrap_or_default();
    let k = top_k.unwrap_or(10);

    // Try to use cached TF-IDF index for this project
    let cache_key = format!("{}:{}", project_id, nodes.len());
    {
        let cache = state.tfidf_cache.lock().map_err(|e| e.to_string())?;
        if let Some(cached) = cache.get(&cache_key) {
            let similar = cached.index.find_similar(&target_content, k);
            let results: Vec<tfidf::TfidfSimilarityResult> = similar.iter().map(|(id, score)| tfidf::TfidfSimilarityResult {
                id: id.clone(),
                name: cached.file_names.get(id).cloned().unwrap_or_default(),
                score: *score,
            }).collect();
            log::info!("TF-IDF (cache-hit) found {} similar files for {}", results.len(), file_id);
            return Ok(results);
        }
    }

    // Cache miss: build index and cache it
    let mut index = tfidf::TfidfIndex::new();
    let mut file_ids = Vec::new();
    let mut file_names: HashMap<String, String> = HashMap::new();

    let max_files = nodes.len().min(500);
    for node in nodes.iter().take(max_files) {
        if node.is_directory { continue; }
        if node.size > 1_000_000 { continue; }
        if let Ok(content) = fs::read_to_string(&node.path) {
            index.add_document(node.id.clone(), content);
            file_ids.push(node.id.clone());
            file_names.insert(node.id.clone(), node.name.clone());
        }
    }
    index.build_idf();

    let similar = index.find_similar(&target_content, k);
    let results: Vec<tfidf::TfidfSimilarityResult> = similar.iter().map(|(id, score)| tfidf::TfidfSimilarityResult {
        id: id.clone(),
        name: file_names.get(id).cloned().unwrap_or_default(),
        score: *score,
    }).collect();

    // Store in cache (key includes node count for invalidation on rescan)
    let cached = CachedTfidf { index, file_ids, file_names };
    state.tfidf_cache.lock().map_err(|e| e.to_string())?.insert(cache_key, cached);

    log::info!("TF-IDF (cache-miss) found {} similar files for {}", results.len(), file_id);
    Ok(results)
}

// Embedding-based similarity command
#[tauri::command]
fn find_similar_by_embedding(project_id: String, file_id: String, top_k: Option<usize>, state: State<AppState>) -> Result<Vec<semantic_embedding::EmbeddingSimilarityResult>, String> {
    let k = top_k.unwrap_or(10);

    // Try to use cached embeddings first (avoids re-reading 500 files per query).
    let (target_emb, candidates, nodes) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let nodes = db.get_file_nodes_by_project(&project_id).map_err(|e| e.to_string())?;
        let target = nodes.iter().find(|n| n.id == file_id)
            .ok_or("File not found")?
            .clone();

        let cached = db.get_project_embeddings(&project_id).map_err(|e| e.to_string())?;
        let cache_map: std::collections::HashMap<String, Vec<f64>> = cached.into_iter().collect();

        if !cache_map.is_empty() {
            let target_emb = if let Some(emb) = cache_map.get(&file_id) {
                emb.clone()
            } else {
                let patterns = semantic_embedding::SemanticPatterns::new();
                let content = fs::read_to_string(&target.path).unwrap_or_default();
                patterns.generate_embedding(&target.name, &target.path, &target.extension, &content)
            };

            let candidates: Vec<(String, Vec<f64>)> = cache_map.into_iter()
                .filter(|(id, _)| id != &file_id)
                .collect();

            (target_emb, candidates, nodes)
        } else {
            // No cache yet — compute all embeddings, then batch-persist in one transaction
            drop(db);
            let patterns = semantic_embedding::SemanticPatterns::new();
            let target_content = fs::read_to_string(&target.path).unwrap_or_default();
            let target_emb = patterns.generate_embedding(&target.name, &target.path, &target.extension, &target_content);

            let max_files = nodes.len().min(500);
            let mut candidates: Vec<(String, Vec<f64>)> = Vec::new();
            let mut embeddings_to_cache: Vec<(String, Vec<f64>)> = Vec::new();

            for node in nodes.iter().take(max_files) {
                if node.id == file_id || node.is_directory { continue; }
                let content = fs::read_to_string(&node.path).unwrap_or_default();
                let emb = patterns.generate_embedding(&node.name, &node.path, &node.extension, &content);
                candidates.push((node.id.clone(), emb.clone()));
                embeddings_to_cache.push((node.id.clone(), emb));
            }

            // Batch-persist all embeddings under a single lock
            embeddings_to_cache.push((file_id.clone(), target_emb.clone()));
            {
                let db = state.db.lock().map_err(|e| e.to_string())?;
                let tx = db.conn.unchecked_transaction().map_err(|e| e.to_string())?;
                for (fid, emb) in &embeddings_to_cache {
                    let emb_json = serde_json::to_string(&emb).unwrap_or_default();
                    let now = chrono::Utc::now().to_rfc3339();
                    let _ = tx.execute(
                        "INSERT OR REPLACE INTO file_embeddings (file_id, embedding, updated_at) VALUES (?1, ?2, ?3)",
                        rusqlite::params![fid, emb_json, now],
                    );
                }
                tx.commit().map_err(|e| format!("Failed to commit embeddings: {}", e))?;
            }

            (target_emb, candidates, nodes)
        }
    };

    let patterns = semantic_embedding::SemanticPatterns::new();
    let similar = patterns.find_similar_by_embedding(&target_emb, &candidates, k);

    let results: Vec<semantic_embedding::EmbeddingSimilarityResult> = similar.iter().map(|(id, score)| {
        let name = nodes.iter().find(|n| n.id == *id).map(|n| n.name.clone()).unwrap_or_default();
        semantic_embedding::EmbeddingSimilarityResult {
            id: id.clone(),
            name,
            score: *score,
        }
    }).collect();

    log::info!("Embedding found {} similar files for {} ({} candidates)", results.len(), file_id, candidates.len());
    Ok(results)
}

// Archive suggestion command
#[tauri::command]
fn suggest_archive_location(project_id: String, file_id: String, state: State<AppState>) -> Result<Vec<semantic_embedding::ArchiveSuggestion>, String> {
    // Fetch node data under lock, then release before file I/O
    let target = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let nodes = db.get_file_nodes_by_project(&project_id).map_err(|e| e.to_string())?;
        nodes.iter().find(|n| n.id == file_id)
            .ok_or("File not found")?
            .clone()
    }; // Lock released here

    let content = fs::read_to_string(&target.path).unwrap_or_default();
    let recommender = semantic_embedding::ArchiveRecommender::new();
    let suggestions = recommender.suggest_archive_location(&target.name, &target.path, &target.extension, &content);

    log::info!("Archive suggested {} locations for {}", suggestions.len(), file_id);
    Ok(suggestions)
}

// AST import parsing command
#[tauri::command]
fn parse_file_imports(path: String) -> Result<Vec<String>, String> {
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let extension = Path::new(&path)
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();

    let imports = ast_parser::parse_imports(&content, &extension);
    log::info!("Parsed {} imports from {}", imports.len(), path);
    Ok(imports)
}

// Import relations analysis command
#[tauri::command]
fn analyze_import_relations(project_id: String, state: State<AppState>) -> Result<Vec<ast_parser::ImportRelationResult>, String> {
    // Fetch node data under lock, then release before file I/O
    let nodes = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_file_nodes_by_project(&project_id).map_err(|e| e.to_string())?
    }; // Lock released here

    let mut results = Vec::new();
    let max_files = nodes.len().min(500);

    for node in nodes.iter().take(max_files) {
        if node.is_directory { continue; }
        if let Ok(content) = fs::read_to_string(&node.path) {
            let extension = node.extension.clone();
            let all_files: Vec<(String, String)> = nodes.iter()
                .filter(|n| !n.is_directory)
                .map(|n| (n.path.clone(), n.name.clone()))
                .collect();

            let relations = ast_parser::analyze_import_relations(&content, &extension, &node.name, &all_files);
            for (target_path, confidence) in relations {
                if let Some(target_node) = nodes.iter().find(|n| n.path == target_path) {
                    results.push(ast_parser::ImportRelationResult {
                        source_id: node.id.clone(),
                        source_name: node.name.clone(),
                        target_id: target_node.id.clone(),
                        target_name: target_node.name.clone(),
                        confidence,
                    });
                }
            }
        }
    }

    log::info!("Import analysis found {} relations for project {}", results.len(), project_id);
    Ok(results)
}

// Plugin list command


// File watcher command
#[tauri::command]
fn start_file_watcher(project_id: String, path: String, app_handle: AppHandle, state: State<AppState>) -> Result<(), String> {
    // Stop any existing watcher for this project first
    {
        let stop_flags = state.watcher_stop_flags.lock().map_err(|e| e.to_string())?;
        if let Some(old_flag) = stop_flags.get(&project_id) {
            old_flag.store(true, Ordering::Relaxed);
        }
        let mut handles = state.watcher_handles.lock().map_err(|e| e.to_string())?;
        if let Some(old_handle) = handles.remove(&project_id) {
            let _ = old_handle.join();
            log::info!("Stopped previous watcher for project {}", project_id);
        }
        state.watcher_stop_flags.lock().map_err(|e| e.to_string())?.remove(&project_id);
    }

    let stop_flag = Arc::new(AtomicBool::new(false));
    let handle = watcher::start_file_watcher(app_handle, project_id.clone(), path, stop_flag.clone())?;
    state.watcher_handles.lock().map_err(|e| e.to_string())?.insert(project_id.clone(), handle);
    state.watcher_stop_flags.lock().map_err(|e| e.to_string())?.insert(project_id.clone(), stop_flag);
    log::info!("Watcher registered for project {}", project_id);
    Ok(())
}

// Stop file watcher command
#[tauri::command]
fn stop_file_watcher(project_id: String, state: State<AppState>) -> Result<(), String> {
    // Signal the thread to stop via the shared flag
    {
        let stop_flags = state.watcher_stop_flags.lock().map_err(|e| e.to_string())?;
        if let Some(flag) = stop_flags.get(&project_id) {
            flag.store(true, Ordering::Relaxed);
        }
    }
    // Wait for the thread to exit (it checks the flag every ~1s)
    let mut handles = state.watcher_handles.lock().map_err(|e| e.to_string())?;
    if let Some(handle) = handles.remove(&project_id) {
        let _ = handle.join();
    }
    state.watcher_stop_flags.lock().map_err(|e| e.to_string())?.remove(&project_id);
    log::info!("Watcher stopped for project {}", project_id);
    Ok(())
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

// Create directory command
#[tauri::command]
async fn create_directory(path: String, project_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let root = get_cached_project_root(&project_id, &state)?;
    validate_path_in_project(&path, &root)?;
    fs::create_dir_all(&path).map_err(|e| format!("Failed to create directory: {}", e))
}

// Move file command
#[tauri::command]
async fn move_file(source: String, destination: String, project_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let root = get_cached_project_root(&project_id, &state)?;
    validate_path_in_project(&source, &root)?;
    validate_path_in_project(&destination, &root)?;
    let src_path = std::path::Path::new(&source);
    let dst_path = std::path::Path::new(&destination);
    
    if !src_path.exists() {
        return Err("Source file does not exist".to_string());
    }
    
    // If destination is a directory, move into it with original name
    let final_dest = if dst_path.is_dir() {
        dst_path.join(src_path.file_name().unwrap_or_default())
    } else {
        dst_path.to_path_buf()
    };
    
    fs::rename(&src_path, &final_dest)
        .map_err(|e| format!("Failed to move file: {}", e))
}

// Copy file command  
#[tauri::command]
async fn copy_file(source: String, destination: String, project_id: String, state: State<'_, AppState>) -> Result<(), String> {
    use std::fs;

    let root = get_cached_project_root(&project_id, &state)?;
    validate_path_in_project(&source, &root)?;
    validate_path_in_project(&destination, &root)?;
    
    let src_path = std::path::Path::new(&source);
    let dst_path = std::path::Path::new(&destination);
    
    if !src_path.exists() {
        return Err("Source file does not exist".to_string());
    }
    
    // If destination is a directory, copy into it with original name
    let final_dest = if dst_path.is_dir() {
        dst_path.join(src_path.file_name().unwrap_or_default())
    } else {
        dst_path.to_path_buf()
    };
    
    if src_path.is_dir() {
        copy_dir_all(src_path, &final_dest)
            .map_err(|e| format!("Failed to copy directory: {}", e))
    } else {
        fs::copy(&src_path, &final_dest)
            .map_err(|e| format!("Failed to copy file: {}", e))?;
        Ok(())
    }
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    use std::fs;
    
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            fs::copy(entry.path(), dest_path)?;
        }
    }
    Ok(())
}

// Trash file command (move to system trash)
#[tauri::command]
async fn trash_file(path: String, project_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let root = get_cached_project_root(&project_id, &state)?;
    validate_path_in_project(&path, &root)?;
    let file_path = std::path::Path::new(&path);
    if !file_path.exists() {
        return Err("File does not exist".to_string());
    }
    
    trash::delete(file_path)
        .map_err(|e| format!("Failed to trash file: {}", e))
}

// ============= Advanced Search =============

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchFilters {
    query: Option<String>,
    file_types: Option<Vec<String>>,      // e.g., ["js", "ts", "tsx"]
    min_size: Option<i64>,                // in bytes
    max_size: Option<i64>,                // in bytes
    modified_after: Option<String>,       // ISO date string
    modified_before: Option<String>,      // ISO date string
    is_directory: Option<bool>,
    tags: Option<Vec<String>>,            // filter by auto-generated tags
}

// ============= Batch Operations =============

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchOperation {
    pub operation: String,  // "move", "copy", "trash", "delete"
    pub paths: Vec<String>,
    pub destination: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchResult {
    pub success: bool,
    pub processed: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

#[tauri::command]
async fn batch_operation(operation: BatchOperation, project_id: String, state: State<'_, AppState>) -> Result<BatchResult, String> {
    let mut processed = 0;
    let mut failed = 0;
    let mut errors: Vec<String> = Vec::new();

    // Path validation is mandatory — project_id is required
    let root = get_cached_project_root(&project_id, &state)?;
    for p in &operation.paths {
        validate_path_in_project(p, &root)?;
    }
    if let Some(ref dest) = operation.destination {
        validate_path_in_project(dest, &root)?;
    }
    
    for path in &operation.paths {
        let result = match operation.operation.as_str() {
            "move" => {
                let dest = operation.destination.as_ref()
                    .ok_or("Destination required for move")?;
                move_file_internal(path, dest)
            }
            "copy" => {
                let dest = operation.destination.as_ref()
                    .ok_or("Destination required for copy")?;
                copy_file_internal(path, dest).await
            }
            "trash" => trash_file_internal(path).await,
            "delete" => delete_file_internal(path).await,
            _ => Err(format!("Unknown operation: {}", operation.operation))
        };
        
        match result {
            Ok(_) => processed += 1,
            Err(e) => {
                failed += 1;
                errors.push(format!("{}: {}", path, e));
            }
        }
    }
    
    log::info!("Batch {} completed: {} success, {} failed", 
        operation.operation, processed, failed);
    
    Ok(BatchResult {
        success: failed == 0,
        processed,
        failed,
        errors,
    })
}

// Helper functions (inline implementations)
fn move_file_internal(source: &str, dest: &str) -> Result<(), String> {
    let src_path = std::path::Path::new(source);
    let dst_path = std::path::Path::new(dest);
    
    if !src_path.exists() {
        return Err("Source does not exist".to_string());
    }
    
    let final_dest = if dst_path.is_dir() {
        dst_path.join(src_path.file_name().unwrap_or_default())
    } else {
        dst_path.to_path_buf()
    };
    
    fs::rename(&src_path, &final_dest)
        .map_err(|e| format!("Failed to move: {}", e))
}

async fn copy_file_internal(source: &str, dest: &str) -> Result<(), String> {
    use std::fs;
    
    let src_path = std::path::Path::new(source);
    let dst_path = std::path::Path::new(dest);
    
    if !src_path.exists() {
        return Err("Source does not exist".to_string());
    }
    
    let final_dest = if dst_path.is_dir() {
        dst_path.join(src_path.file_name().unwrap_or_default())
    } else {
        dst_path.to_path_buf()
    };
    
    if src_path.is_dir() {
        copy_dir_all(src_path, &final_dest)
            .map_err(|e| format!("Failed to copy directory: {}", e))
    } else {
        fs::copy(&src_path, &final_dest)
            .map_err(|e| format!("Failed to copy file: {}", e))?;
        Ok(())
    }
}

async fn trash_file_internal(path: &str) -> Result<(), String> {
    let file_path = std::path::Path::new(path);
    if !file_path.exists() {
        return Err("File does not exist".to_string());
    }
    trash::delete(file_path)
        .map_err(|e| format!("Failed to trash: {}", e))
}

async fn delete_file_internal(path: &str) -> Result<(), String> {
    let file_path = std::path::Path::new(path);
    if !file_path.exists() {
        return Err("File does not exist".to_string());
    }
    
    if file_path.is_dir() {
        fs::remove_dir_all(path).map_err(|e| format!("Failed to delete: {}", e))
    } else {
        fs::remove_file(path).map_err(|e| format!("Failed to delete: {}", e))
    }
}

// ============= Backup/Sync =============

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectExport {
    pub project: Project,
    pub nodes: Vec<FileNode>,
    pub relations: Vec<db::FileRelation>,
    pub favorites: Vec<String>, // file_ids
    pub tags: Vec<db::Tag>,
    pub file_tags: Vec<(String, String)>, // (file_id, tag_id)
}

#[tauri::command]
fn export_project(project_id: String, state: State<AppState>) -> Result<ProjectExport, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let project = db.get_project(&project_id)
        .map_err(|e| e.to_string())?
        .ok_or("Project not found")?;

    let nodes = db.get_file_nodes_by_project(&project_id)
        .map_err(|e| e.to_string())?;

    let relations = db.get_relations_by_project(&project_id)
        .map_err(|e| e.to_string())?;

    // Get favorites
    let favorites = db.get_favorites(&project_id)
        .map_err(|e| e.to_string())?;

    // P2-3: Only export tags associated with this project's files (not global tags)
    let tags = db.list_tags_by_project(&project_id)
        .map_err(|e| e.to_string())?;
    
    // Get all file tags
    let mut file_tags: Vec<(String, String)> = Vec::new();
    for node in &nodes {
        if let Ok(node_tags) = db.get_file_tags(&node.id) {
            for tag in node_tags {
                file_tags.push((node.id.clone(), tag.id));
            }
        }
    }
    
    log::info!("Exported project {}: {} nodes, {} relations", 
        project_id, nodes.len(), relations.len());
    
    Ok(ProjectExport {
        project,
        nodes,
        relations,
        favorites,
        tags,
        file_tags,
    })
}

#[tauri::command]
fn import_project(data: ProjectExport, state: State<AppState>) -> Result<Project, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    
    // Create new project with new name to avoid conflict
    let new_name = format!("{} (导入)", data.project.name);
    let project = db.create_project(&new_name, &data.project.root_path)
        .map_err(|e| e.to_string())?;
    
    // Create id mapping for nodes
    let mut node_id_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    
    // Import nodes
    for node in data.nodes {
        let new_id = uuid::Uuid::new_v4().to_string();
        node_id_map.insert(node.id.clone(), new_id.clone());

        let new_node = FileNode {
            id: new_id,
            project_id: project.id.clone(),
            path: node.path,
            name: node.name,
            extension: node.extension,
            size: node.size,
            created_at: node.created_at,
            modified_at: node.modified_at,
            tags: node.tags,
            parent_id: node.parent_id,
            position_x: node.position_x,
            position_y: node.position_y,
            is_collapsed: node.is_collapsed,
            is_directory: node.is_directory,
            children: node.children,
            related_files: node.related_files,
        };
        db.insert_file_node(&new_node).map_err(|e| format!("Failed to import node: {}", e))?;
    }

    // Create tags first
    let mut tag_id_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for tag in &data.tags {
        let new_tag = db.create_tag(&tag.name, &tag.color)
            .map_err(|e| e.to_string())?;
        tag_id_map.insert(tag.id.clone(), new_tag.id);
    }

    // P2-2: Import favorites using node_id_map to remap file_ids
    for file_id in data.favorites {
        if let Some(new_file_id) = node_id_map.get(&file_id) {
            db.add_favorite(&project.id, new_file_id).map_err(|e| format!("Failed to import favorite: {}", e))?;
        }
    }

    // Import file tags (remap both file_id and tag_id)
    for (file_id, tag_id) in data.file_tags {
        let remapped_file_id = node_id_map.get(&file_id);
        let remapped_tag_id = tag_id_map.get(&tag_id);
        if let (Some(new_file_id), Some(new_tag_id)) = (remapped_file_id, remapped_tag_id) {
            db.add_file_tag(new_file_id, new_tag_id).map_err(|e| format!("Failed to import file tag: {}", e))?;
        }
    }
    
    log::info!("Imported project {} from backup", project.id);
    Ok(project)
}

#[tauri::command]
fn get_project_stats(project_id: String, state: State<AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    
    let nodes = db.get_file_nodes_by_project(&project_id)
        .map_err(|e| e.to_string())?;
    
    let total_files = nodes.iter().filter(|n| !n.is_directory).count();
    let total_dirs = nodes.iter().filter(|n| n.is_directory).count();
    let total_size: i64 = nodes.iter().map(|n| n.size).sum();
    
    let favorites = db.get_favorites(&project_id)
        .map_err(|e| e.to_string())?;
    
    let stats = serde_json::json!({
        "totalFiles": total_files,
        "totalDirs": total_dirs,
        "totalSize": total_size,
        "favoriteCount": favorites.len(),
        "nodeCount": nodes.len(),
    });
    
    Ok(stats)
}

#[tauri::command]
fn advanced_search(
    project_id: String, 
    filters: SearchFilters, 
    state: State<AppState>
) -> Result<Vec<FileNode>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut nodes = db.get_file_nodes_by_project(&project_id).map_err(|e| e.to_string())?;
    
    // Apply filters
    if let Some(ref query) = filters.query {
        if !query.is_empty() {
            let query_lower = query.to_lowercase();
            nodes.retain(|n| {
                n.name.to_lowercase().contains(&query_lower) || 
                n.path.to_lowercase().contains(&query_lower) ||
                n.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            });
        }
    }
    
    // Filter by file types (extensions)
    if let Some(ref types) = filters.file_types {
        if !types.is_empty() {
            let types_lower: Vec<String> = types.iter().map(|t| t.to_lowercase()).collect();
            nodes.retain(|n| {
                if n.is_directory {
                    types_lower.iter().any(|t| t == "folder" || t == "directory")
                } else {
                    types_lower.iter().any(|t| n.extension.to_lowercase() == *t)
                }
            });
        }
    }
    
    // Filter by size
    if let Some(min_size) = filters.min_size {
        nodes.retain(|n| n.size >= min_size);
    }
    if let Some(max_size) = filters.max_size {
        nodes.retain(|n| n.size <= max_size);
    }
    
    // Filter by modification date
    if let Some(ref after) = filters.modified_after {
        match chrono::DateTime::parse_from_rfc3339(after) {
            Ok(after_date) => {
                nodes.retain(|n| {
                    n.modified_at.as_ref().and_then(|m| chrono::DateTime::parse_from_rfc3339(m).ok())
                        .map_or(false, |mod_date| mod_date >= after_date)
                });
            }
            Err(_) => log::warn!("advanced_search: failed to parse modified_after date: {}", after),
        }
    }
    if let Some(ref before) = filters.modified_before {
        match chrono::DateTime::parse_from_rfc3339(before) {
            Ok(before_date) => {
                nodes.retain(|n| {
                    n.modified_at.as_ref().and_then(|m| chrono::DateTime::parse_from_rfc3339(m).ok())
                        .map_or(false, |mod_date| mod_date <= before_date)
                });
            }
            Err(_) => log::warn!("advanced_search: failed to parse modified_before date: {}", before),
        }
    }
    
    // Filter by is_directory
    if let Some(is_dir) = filters.is_directory {
        nodes.retain(|n| n.is_directory == is_dir);
    }
    
    // Filter by tags
    if let Some(ref search_tags) = filters.tags {
        if !search_tags.is_empty() {
            let tags_lower: Vec<String> = search_tags.iter().map(|t| t.to_lowercase()).collect();
            nodes.retain(|n| {
                n.tags.iter().any(|t| tags_lower.contains(&t.to_lowercase()))
            });
        }
    }
    
    log::info!("Advanced search found {} results for project {}", nodes.len(), project_id);
    Ok(nodes)
}

// Delete file permanently
#[tauri::command]
async fn delete_file_permanent(path: String, project_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let root = get_cached_project_root(&project_id, &state)?;
    validate_path_in_project(&path, &root)?;
    let file_path = std::path::Path::new(&path);
    if !file_path.exists() {
        return Err("File does not exist".to_string());
    }
    
    if file_path.is_dir() {
        fs::remove_dir_all(&path).map_err(|e| format!("Failed to delete directory: {}", e))
    } else {
        fs::remove_file(&path).map_err(|e| format!("Failed to delete file: {}", e))
    }
}

// ============= Favorites Commands =============

#[tauri::command]
fn add_favorite(project_id: String, file_id: String, state: State<AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.add_favorite(&project_id, &file_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_favorite(project_id: String, file_id: String, state: State<AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.remove_favorite(&project_id, &file_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_favorites(project_id: String, state: State<AppState>) -> Result<Vec<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_favorites(&project_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn is_favorite(project_id: String, file_id: String, state: State<AppState>) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.is_favorite(&project_id, &file_id).map_err(|e| e.to_string())
}

// ============= Tags Commands =============

#[tauri::command]
fn create_tag(name: String, color: String, state: State<AppState>) -> Result<db::Tag, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.create_tag(&name, &color).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_tags(state: State<AppState>) -> Result<Vec<db::Tag>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_tags().map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_tag(id: String, state: State<AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_tag(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_file_tag(file_id: String, tag_id: String, state: State<AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.add_file_tag(&file_id, &tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_file_tag(file_id: String, tag_id: String, state: State<AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.remove_file_tag(&file_id, &tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_file_tags(file_id: String, state: State<AppState>) -> Result<Vec<db::Tag>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_file_tags(&file_id).map_err(|e| e.to_string())
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
                watcher_handles: Mutex::new(HashMap::new()),
                watcher_stop_flags: Mutex::new(HashMap::new()),
                project_roots: RwLock::new(HashMap::new()),
                tfidf_cache: Mutex::new(HashMap::new()),
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
            semantic_search_files,
            find_similar_by_content,
            find_similar_by_embedding,
            suggest_archive_location,
            parse_file_imports,
            analyze_import_relations,
            start_file_watcher,
            stop_file_watcher,
            open_directory_dialog,
            create_directory,
            move_file,
            copy_file,
            trash_file,
            delete_file_permanent,
            // Favorites
            add_favorite,
            remove_favorite,
            get_favorites,
            is_favorite,
            // Tags
            create_tag,
            list_tags,
            delete_tag,
            add_file_tag,
            remove_file_tag,
            get_file_tags,
            // Advanced Search
            advanced_search,
            // Batch Operations
            batch_operation,
            // Backup/Sync
            export_project,
            import_project,
            get_project_stats,
            // File Edit History
            get_file_history,
            restore_file_version,
            delete_file_history_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
