use notify::{Watcher, RecursiveMode, Event, Config};
use std::sync::mpsc::channel;
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::db::Database;

// File watcher event types for frontend
#[derive(Clone, serde::Serialize)]
pub struct FileChangeEvent {
    pub event_type: String,
    pub path: String,
    pub is_directory: bool,
}

// Scan a single directory and return file nodes
pub fn scan_path(path: &Path, project_id: &str) -> Vec<crate::db::FileNode> {
    let mut nodes = Vec::new();
    let root_path = path.to_string_lossy().to_string();
    
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
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
        
        let parent_path = entry_path.parent();
        let parent_id = if let Some(parent) = parent_path {
            if parent.to_string_lossy().to_string() == root_path {
                None
            } else {
                None
            }
        } else {
            None
        };
        
        let position_index = nodes.len() as f64;
        let x = (position_index % 5.0) * 250.0;
        let y = (position_index / 5.0) * 150.0;
        
        let node = crate::db::FileNode {
            id: Uuid::new_v4().to_string(),
            project_id: project_id.to_string(),
            path: entry_path.to_string_lossy().to_string(),
            name,
            extension,
            size,
            created_at,
            modified_at,
            tags: vec![],
            parent_id,
            position_x: x,
            position_y: y,
            is_collapsed: false,
            is_directory: is_dir,
        };
        
        nodes.push(node);
    }
    
    nodes
}

// Start file watcher for a project directory
pub fn start_file_watcher(
    app_handle: AppHandle,
    project_id: String,
    path: String,
) -> Result<(), String> {
    let path_clone = path.clone();
    
    std::thread::spawn(move || {
        let (tx, rx) = channel();
        
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        }).expect("Failed to create watcher");
        
        watcher.watch(Path::new(&path_clone), RecursiveMode::Recursive)
            .expect("Failed to start watching");
        
        log::info!("Started watching: {}", path_clone);
        
        loop {
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(event) => {
                    for path in event.paths {
                        let event_type = match event.kind {
                            notify::EventKind::Create(_) => "created",
                            notify::EventKind::Modify(_) => "modified",
                            notify::EventKind::Remove(_) => "deleted",
                            _ => continue,
                        };
                        
                        let is_dir = path.is_dir();
                        
                        let change_event = FileChangeEvent {
                            event_type: event_type.to_string(),
                            path: path.to_string_lossy().to_string(),
                            is_directory: is_dir,
                        };
                        
                        let _ = app_handle.emit("file-change", &change_event);
                        log::info!("File change: {} - {}", event_type, path.display());
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(_) => break,
            }
        }
    });
    
    Ok(())
}

// AI Agent: Analyze file relations based on extension and naming patterns
pub fn analyze_relations(nodes: &[crate::db::FileNode]) -> Vec<crate::db::FileRelation> {
    let mut relations = Vec::new();
    
    for (i, node1) in nodes.iter().enumerate() {
        for node2 in nodes.iter().skip(i + 1) {
            if let Some((rel_type, confidence)) = determine_relation(node1, node2) {
                relations.push(crate::db::FileRelation {
                    id: Uuid::new_v4().to_string(),
                    project_id: node1.project_id.clone(),
                    source_id: node1.id.clone(),
                    target_id: node2.id.clone(),
                    relation_type: rel_type,
                    confidence,
                });
            }
        }
    }
    
    relations
}

fn determine_relation(node1: &crate::db::FileNode, node2: &crate::db::FileNode) -> Option<(String, f64)> {
    if node1.extension == node2.extension && !node1.extension.is_empty() {
        return Some(("auto".to_string(), 0.6));
    }
    
    let name1 = node1.name.to_lowercase();
    let name2 = node2.name.to_lowercase();
    
    if name1.contains("index") && node1.extension == node2.extension {
        return Some(("import".to_string(), 0.8));
    }
    
    let name_without_ext1 = node1.name.trim_end_matches(&format!(".{}", node1.extension));
    let name_without_ext2 = node2.name.trim_end_matches(&format!(".{}", node2.extension));
    
    if name_without_ext1 == name_without_ext2 && is_style_extension(&node1.extension) && is_style_extension(&node2.extension) {
        return Some(("similar".to_string(), 0.7));
    }
    
    if (name1.contains("test") || name1.contains("spec")) && (name2.contains("test") || name2.contains("spec")) {
        return Some(("similar".to_string(), 0.5));
    }
    
    if node1.path.contains(&node2.name) || node2.path.contains(&node1.name) {
        return Some(("auto".to_string(), 0.4));
    }
    
    None
}

fn is_style_extension(ext: &str) -> bool {
    matches!(ext.to_lowercase().as_str(), "css" | "scss" | "less" | "sass")
}

// AI Agent: Generate tags based on file properties
pub fn generate_file_tags(node: &crate::db::FileNode) -> Vec<String> {
    let mut tags = Vec::new();
    let ext = node.extension.to_lowercase();
    let name = node.name.to_lowercase();
    
    match ext.as_str() {
        "js" | "jsx" | "ts" | "tsx" | "mjs" => tags.push("JavaScript"),
        "py" => tags.push("Python"),
        "rs" => tags.push("Rust"),
        "go" => tags.push("Go"),
        "java" => tags.push("Java"),
        "c" | "cpp" | "h" | "hpp" => tags.push("C/C++"),
        "cs" => tags.push("C#"),
        "swift" => tags.push("Swift"),
        "kt" => tags.push("Kotlin"),
        "php" => tags.push("PHP"),
        "rb" => tags.push("Ruby"),
        "html" | "htm" => tags.push("Web"),
        "css" | "scss" | "less" | "sass" => tags.push("Styles"),
        "json" => tags.push("Config"),
        "xml" => tags.push("XML"),
        "yaml" | "yml" => tags.push("YAML"),
        "md" | "markdown" => tags.push("文档"),
        "txt" => tags.push("文本"),
        "pdf" => tags.push("PDF"),
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "ico" => tags.push("图片"),
        "mp4" | "webm" | "mkv" | "mov" => tags.push("视频"),
        "mp3" | "wav" | "ogg" | "flac" => tags.push("音频"),
        "zip" | "rar" | "7z" | "tar" | "gz" => tags.push("压缩包"),
        _ => {}
    }
    
    if name.contains("test") || name.contains("spec") {
        tags.push("测试");
    }
    if name.contains("config") || name.contains("cfg") {
        tags.push("配置");
    }
    if name.contains("readme") || name.contains("changelog") {
        tags.push("文档");
    }
    if name.starts_with('.') {
        tags.push("隐藏文件");
    }
    
    if node.size > 10 * 1024 * 1024 {
        tags.push("大文件");
    } else if node.size < 1024 {
        tags.push("小文件");
    }
    
    if node.is_directory {
        tags.push("目录");
    }
    
    tags
}

// Search files by name or tag
pub fn search_files(nodes: &[crate::db::FileNode], query: &str) -> Vec<crate::db::FileNode> {
    let query_lower = query.to_lowercase();
    
    nodes.iter()
        .filter(|node| {
            node.name.to_lowercase().contains(&query_lower) ||
            node.tags.iter().any(|t| t.to_lowercase().contains(&query_lower)) ||
            node.extension.to_lowercase().contains(&query_lower)
        })
        .cloned()
        .collect()
}

// Find similar files
pub fn find_similar_files(nodes: &[crate::db::FileNode], target: &crate::db::FileNode) -> Vec<crate::db::FileNode> {
    nodes.iter()
        .filter(|node| {
            node.id != target.id && (
                node.extension == target.extension ||
                node.path.parent() == target.path.parent() ||
                similarity(&node.name, &target.name) > 0.5
            )
        })
        .cloned()
        .collect()
}

fn similarity(s1: &str, s2: &str) -> f64 {
    let s1_lower = s1.to_lowercase();
    let s2_lower = s2.to_lowercase();
    
    if s1_lower == s2_lower {
        return 1.0;
    }
    
    if s1_lower.contains(&s2_lower) || s2_lower.contains(&s1_lower) {
        let shorter = s1_lower.len().min(s2_lower.len());
        let longer = s1_lower.len().max(s2_lower.len());
        return shorter as f64 / longer as f64;
    }
    
    let chars1: Vec<char> = s1_lower.chars().collect();
    let chars2: Vec<char> = s2_lower.chars().collect();
    
    let mut matches = 0;
    for c in &chars1 {
        if chars2.contains(c) {
            matches += 1;
        }
    }
    
    let total = (chars1.len() + chars2.len()) / 2;
    if total == 0 {
        return 0.0;
    }
    
    matches as f64 / total as f64
}
