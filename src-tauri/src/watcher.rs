use notify::{Watcher, RecursiveMode, Event, Config};
use std::sync::mpsc::channel;
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
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
// Limited to first 5000 nodes to prevent O(n²) blowup
pub fn analyze_relations(nodes: &[crate::db::FileNode]) -> Vec<crate::db::FileRelation> {
    let mut relations = Vec::new();
    let max_nodes = nodes.len().min(5000);

    for (i, node1) in nodes[..max_nodes].iter().enumerate() {
        for node2 in nodes[..max_nodes].iter().skip(i + 1) {
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
        "js" | "jsx" | "ts" | "tsx" | "mjs" => tags.push("JavaScript".to_string()),
        "py" => tags.push("Python".to_string()),
        "rs" => tags.push("Rust".to_string()),
        "go" => tags.push("Go".to_string()),
        "java" => tags.push("Java".to_string()),
        "c" | "cpp" | "h" | "hpp" => tags.push("C/C++".to_string()),
        "cs" => tags.push("C#".to_string()),
        "swift" => tags.push("Swift".to_string()),
        "kt" => tags.push("Kotlin".to_string()),
        "php" => tags.push("PHP".to_string()),
        "rb" => tags.push("Ruby".to_string()),
        "html" | "htm" => tags.push("Web".to_string()),
        "css" | "scss" | "less" | "sass" => tags.push("Styles".to_string()),
        "json" => tags.push("Config".to_string()),
        "xml" => tags.push("XML".to_string()),
        "yaml" | "yml" => tags.push("YAML".to_string()),
        "md" | "markdown" => tags.push("文档".to_string()),
        "txt" => tags.push("文本".to_string()),
        "pdf" => tags.push("PDF".to_string()),
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "ico" => tags.push("图片".to_string()),
        "mp4" | "webm" | "mkv" | "mov" => tags.push("视频".to_string()),
        "mp3" | "wav" | "ogg" | "flac" => tags.push("音频".to_string()),
        "zip" | "rar" | "7z" | "tar" | "gz" => tags.push("压缩包".to_string()),
        _ => {}
    }

    if name.contains("test") || name.contains("spec") {
        tags.push("测试".to_string());
    }
    if name.contains("config") || name.contains("cfg") {
        tags.push("配置".to_string());
    }
    if name.contains("readme") || name.contains("changelog") {
        tags.push("文档".to_string());
    }
    if name.starts_with('.') {
        tags.push("隐藏文件".to_string());
    }

    if node.size > 10 * 1024 * 1024 {
        tags.push("大文件".to_string());
    } else if node.size < 1024 {
        tags.push("小文件".to_string());
    }

    if node.is_directory {
        tags.push("目录".to_string());
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
    let target_path = std::path::Path::new(&target.path);
    nodes.iter()
        .filter(|node| {
            let node_path = std::path::Path::new(&node.path);
            node.id != target.id && (
                node.extension == target.extension ||
                node_path.parent() == target_path.parent() ||
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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::FileNode;

    fn make_node(id: &str, name: &str, ext: &str, path: &str, size: i64, is_dir: bool) -> FileNode {
        FileNode {
            id: id.to_string(),
            project_id: "test-proj".to_string(),
            path: path.to_string(),
            name: name.to_string(),
            extension: ext.to_string(),
            size,
            created_at: None,
            modified_at: None,
            tags: vec![],
            parent_id: None,
            position_x: 0.0,
            position_y: 0.0,
            is_collapsed: false,
            is_directory: is_dir,
        }
    }

    #[test]
    fn test_generate_file_tags_js() {
        let node = make_node("1", "app.js", "js", "/test/app.js", 500, false);
        let tags = generate_file_tags(&node);
        assert!(tags.contains(&"JavaScript".to_string()));
    }

    #[test]
    fn test_generate_file_tags_python() {
        let node = make_node("2", "main.py", "py", "/test/main.py", 500, false);
        let tags = generate_file_tags(&node);
        assert!(tags.contains(&"Python".to_string()));
    }

    #[test]
    fn test_generate_file_tags_image() {
        let node = make_node("3", "photo.png", "png", "/test/photo.png", 500, false);
        let tags = generate_file_tags(&node);
        assert!(tags.contains(&"图片".to_string()));
    }

    #[test]
    fn test_generate_file_tags_large_file() {
        let node = make_node("4", "big.bin", "bin", "/test/big.bin", 20 * 1024 * 1024, false);
        let tags = generate_file_tags(&node);
        assert!(tags.contains(&"大文件".to_string()));
    }

    #[test]
    fn test_generate_file_tags_small_file() {
        let node = make_node("5", "tiny.txt", "txt", "/test/tiny.txt", 100, false);
        let tags = generate_file_tags(&node);
        assert!(tags.contains(&"小文件".to_string()));
    }

    #[test]
    fn test_generate_file_tags_test_file() {
        let node = make_node("6", "app.test.ts", "ts", "/test/app.test.ts", 500, false);
        let tags = generate_file_tags(&node);
        assert!(tags.contains(&"测试".to_string()));
    }

    #[test]
    fn test_generate_file_tags_hidden_file() {
        let node = make_node("7", ".env", "", "/test/.env", 100, false);
        let tags = generate_file_tags(&node);
        assert!(tags.contains(&"隐藏文件".to_string()));
    }

    #[test]
    fn test_generate_file_tags_directory() {
        let node = make_node("8", "src", "", "/test/src", 4096, true);
        let tags = generate_file_tags(&node);
        assert!(tags.contains(&"目录".to_string()));
    }

    #[test]
    fn test_search_files_by_name() {
        let nodes = vec![
            make_node("1", "main.ts", "ts", "/main.ts", 100, false),
            make_node("2", "utils.ts", "ts", "/utils.ts", 100, false),
            make_node("3", "readme.md", "md", "/readme.md", 100, false),
        ];
        let results = search_files(&nodes, "main");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "main.ts");
    }

    #[test]
    fn test_search_files_by_extension() {
        let nodes = vec![
            make_node("1", "main.ts", "ts", "/main.ts", 100, false),
            make_node("2", "utils.ts", "ts", "/utils.ts", 100, false),
            make_node("3", "readme.md", "md", "/readme.md", 100, false),
        ];
        let results = search_files(&nodes, "ts");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_files_by_tag() {
        let mut node = make_node("1", "main.ts", "ts", "/main.ts", 100, false);
        node.tags = vec!["JavaScript".to_string()];
        let nodes = vec![node];
        let results = search_files(&nodes, "javascript");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_files_no_match() {
        let nodes = vec![
            make_node("1", "main.ts", "ts", "/main.ts", 100, false),
        ];
        let results = search_files(&nodes, "nonexistent");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_files_case_insensitive() {
        let nodes = vec![
            make_node("1", "MainFile.ts", "ts", "/MainFile.ts", 100, false),
        ];
        let results = search_files(&nodes, "mainfile");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_analyze_relations_same_extension() {
        let nodes = vec![
            make_node("1", "a.ts", "ts", "/a.ts", 100, false),
            make_node("2", "b.ts", "ts", "/b.ts", 100, false),
        ];
        let rels = analyze_relations(&nodes);
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].relation_type, "auto");
        assert!((rels[0].confidence - 0.6).abs() < 0.001);
    }

    #[test]
    fn test_analyze_relations_test_files() {
        let nodes = vec![
            make_node("1", "a.test.ts", "ts", "/a.test.ts", 100, false),
            make_node("2", "b.spec.ts", "ts", "/b.spec.ts", 100, false),
        ];
        let rels = analyze_relations(&nodes);
        // a.test.ts and b.spec.ts have same extension, so only 1 relation (auto)
        // The test file check doesn't trigger because same_extension check returns early
        assert_eq!(rels.len(), 1);
    }

    #[test]
    fn test_analyze_relations_empty() {
        let rels = analyze_relations(&[]);
        assert_eq!(rels.len(), 0);
    }

    #[test]
    fn test_analyze_relations_single_node() {
        let nodes = vec![
            make_node("1", "a.ts", "ts", "/a.ts", 100, false),
        ];
        let rels = analyze_relations(&nodes);
        assert_eq!(rels.len(), 0);
    }

    #[test]
    fn test_find_similar_files_same_extension() {
        let target = make_node("1", "a.ts", "ts", "/src/a.ts", 100, false);
        let nodes = vec![
            target.clone(),
            make_node("2", "b.ts", "ts", "/src/b.ts", 100, false),
            make_node("3", "c.md", "md", "/docs/c.md", 100, false),
        ];
        let similar = find_similar_files(&nodes, &target);
        // b.ts matches by extension, c.md is in different directory so doesn't match
        assert_eq!(similar.len(), 1);
        assert_eq!(similar[0].name, "b.ts");
    }

    #[test]
    fn test_find_similar_files_self_excluded() {
        let target = make_node("1", "a.ts", "ts", "/a.ts", 100, false);
        let nodes = vec![target.clone()];
        let similar = find_similar_files(&nodes, &target);
        assert_eq!(similar.len(), 0);
    }

    #[test]
    fn test_similarity_identical() {
        assert!((similarity("hello", "hello") - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_similarity_contains() {
        let s = similarity("component", "component.test");
        assert!(s > 0.5 && s < 1.0);
    }

    #[test]
    fn test_similarity_no_match() {
        let s = similarity("abc", "xyz");
        assert!(s < 0.5);
    }
}
