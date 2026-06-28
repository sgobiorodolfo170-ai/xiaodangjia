use crate::db::FileNode;

/// Generate tags based on file properties.
pub fn generate_file_tags(node: &FileNode) -> Vec<String> {
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
    fn test_generate_file_tags_js() {
        let node = make_node("1", "app.js", "js", "/test/app.js", 500, false);
        let tags = generate_file_tags(&node);
        assert!(tags.contains(&"JavaScript".to_string()));
    }

    #[test]
    fn test_generate_file_tags_directory() {
        let node = make_node("2", "src", "", "/test/src", 4096, true);
        let tags = generate_file_tags(&node);
        assert!(tags.contains(&"目录".to_string()));
    }
}
