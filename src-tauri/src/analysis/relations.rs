use uuid::Uuid;

use crate::db::{Database, FileNode, FileRelation};

/// Analyze file relations based on extension and naming patterns.
/// Limited to first 5000 nodes to prevent O(n²) blowup.
pub fn analyze_relations(nodes: &[FileNode]) -> Vec<FileRelation> {
    let mut relations = Vec::new();
    let max_nodes = nodes.len().min(5000);

    for (i, node1) in nodes[..max_nodes].iter().enumerate() {
        for node2 in nodes[..max_nodes].iter().skip(i + 1) {
            if let Some((rel_type, confidence)) = determine_relation(node1, node2) {
                relations.push(FileRelation {
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

fn determine_relation(node1: &FileNode, node2: &FileNode) -> Option<(String, f64)> {
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
    fn test_analyze_relations_same_extension() {
        let nodes = vec![
            make_node("1", "a.ts", "ts", "/a.ts", 100, false),
            make_node("2", "b.ts", "ts", "/b.ts", 100, false),
        ];
        let rels = analyze_relations(&nodes);
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].relation_type, "auto");
    }

    #[test]
    fn test_analyze_relations_empty() {
        let rels = analyze_relations(&[]);
        assert_eq!(rels.len(), 0);
    }
}
