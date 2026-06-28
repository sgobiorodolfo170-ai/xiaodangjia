use crate::db::FileNode;

/// Search files by name, tag, or extension.
pub fn search_files(nodes: &[FileNode], query: &str) -> Vec<FileNode> {
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

/// Find similar files by extension, directory, or name similarity.
pub fn find_similar_files(nodes: &[FileNode], target: &FileNode) -> Vec<FileNode> {
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

/// Simple string similarity based on character overlap.
pub fn similarity(s1: &str, s2: &str) -> f64 {
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
            children: Vec::new(),
            related_files: Vec::new(),
        }
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
    }

    #[test]
    fn test_similarity_identical() {
        assert!((similarity("hello", "hello") - 1.0).abs() < 0.001);
    }
}
