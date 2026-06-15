---
title: 数据标注与文件元数据规范
description: 小当家项目的文件元数据标注规范
---

# 数据标注与文件元数据规范

## 1. 规范概述

本文档定义小当家项目中文件元数据的标注规范，包括自动生成标签的规则、数据格式要求等。

## 2. 标签体系

### 2.1 标签分类

| 类别 | 说明 | 示例 |
|------|------|------|
| 语言/技术 | 编程语言或技术栈 | JavaScript, Python, Rust |
| 文件类型 | 文件类型分类 | 文档, 图片, 视频, 配置 |
| 角色 | 在项目中的角色 | 测试, 入口文件, 隐藏文件 |
| 状态 | 文件状态特征 | 大文件, 超大文件, 新建 |
| 自定义 | 用户添加的标签 | 用户自定义 |

### 2.2 基于扩展名的标签映射

```rust
// 编程语言/技术栈
let ext_to_lang = HashMap::from([
    ("js", "JavaScript"),
    ("jsx", "JavaScript"),
    ("ts", "TypeScript"),
    ("tsx", "TypeScript"),
    ("py", "Python"),
    ("rs", "Rust"),
    ("go", "Go"),
    ("java", "Java"),
    ("c", "C"),
    ("cpp", "C++"),
    ("h", "C/C++头文件"),
    ("hpp", "C++头文件"),
    ("cs", "C#"),
    ("swift", "Swift"),
    ("kt", "Kotlin"),
    ("scala", "Scala"),
    ("rb", "Ruby"),
    ("php", "PHP"),
    ("lua", "Lua"),
]);

// 配置文件
let config_exts = ["json", "xml", "yaml", "yml", "toml", "ini", "sql", "env"];

// 文档格式
let doc_exts = ["md", "txt", "rst", "tex"];

// 图片格式
let image_exts = ["png", "jpg", "jpeg", "gif", "bmp", "webp", "svg", "ico", "tiff", "psd", "ai", "eps"];

// 视频格式
let video_exts = ["mp4", "webm", "mkv", "avi", "mov", "wmv", "flv", "m4v", "mpg", "mpeg", "3gp", "ogv"];

// 音频格式
let audio_exts = ["mp3", "wav", "ogg", "flac", "aac", "m4a", "wma", "opus", "ape", "mid", "midi"];

// 压缩包格式
let archive_exts = ["zip", "rar", "7z", "tar", "gz", "bz2", "xz", "iso", "dmg", "deb", "rpm"];

// Office 文档
let office_exts = ["doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods", "odp", "rtf", "csv"];

// 电子书格式
let ebook_exts = ["epub", "mobi", "azw3", "djvu", "djv", "chm", "cbr", "cbz", "fb2", "lit"];
```

### 2.3 基于文件名的关键词匹配

```rust
// 测试文件关键词
let test_keywords = ["test", "spec", "__tests__", "tests", "test_", "_test"];

// 配置文件关键词
let config_keywords = ["config", ".env", ".gitignore", "dockerfile", "makefile"];

// 入口文件关键词
let entry_keywords = ["main", "index", "app", "server", "entry"];

// 构建文件关键词
let build_keywords = ["build", "webpack", "vite", "rollup", "esbuild", "tsc"];
```

## 3. 元数据格式

### 3.1 文件节点 JSON 格式

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "project_id": "550e8400-e29b-41d4-a716-446655440001",
  "path": "C:\\Projects\\MyApp\\src\\main.ts",
  "name": "main.ts",
  "extension": "ts",
  "size": 2048,
  "created_at": "2024-01-15T10:30:00Z",
  "modified_at": "2024-06-01T14:22:00Z",
  "tags": ["TypeScript", "入口文件", "源代码"],
  "parent_id": "550e8400-e29b-41d4-a716-446655440002",
  "position_x": 100.0,
  "position_y": 200.0,
  "is_collapsed": false,
  "is_directory": false
}
```

### 3.2 标签存储格式

标签以 JSON 数组形式存储在 SQLite 中:

```sql
-- 存储
INSERT INTO file_nodes (id, tags) VALUES ('...', '["JavaScript", "测试"]');

-- 读取
SELECT json(tags) FROM file_nodes WHERE id = '...';
```

## 4. 自动标签生成规则

### 4.1 优先级规则

| 优先级 | 标签来源 | 说明 |
|--------|---------|------|
| P0 | 扩展名 | 根据文件扩展名生成 |
| P1 | 文件名 | 根据文件名关键词匹配 |
| P2 | 文件大小 | 超过阈值添加标记 |
| P3 | 目录属性 | 是否为隐藏文件、目录等 |
| P4 | AI 生成 | 未来由 AI 模型生成 |

### 4.2 大小阈值定义

| 阈值 | 标签 |
|------|------|
| > 100 MB | 超大文件 |
| > 10 MB | 大文件 |
| > 1 MB | 中等文件 |
| <= 1 MB | 小文件 |

### 4.3 特殊标记规则

| 条件 | 标签 |
|------|------|
| 文件名以 `.` 开头 | 隐藏文件 |
| 是目录 | 目录 |
| 包含 test/spec 关键词 | 测试文件 |
| 包含 config 关键词 | 配置文件 |

## 5. 数据质量要求

### 5.1 必填字段

- `id`: 全局唯一标识
- `project_id`: 所属项目
- `path`: 文件路径
- `name`: 文件名
- `extension`: 扩展名 (空字符串表示无扩展名)

### 5.2 可选字段

- `tags`: 默认为空数组 `[]`
- `parent_id`: 根目录为 null
- `position_x/y`: 默认为 0

### 5.3 字段校验规则

| 字段 | 校验规则 |
|------|---------|
| id | UUID v4 格式 |
| name | 非空，最大 255 字符 |
| extension | 最大 20 字符 |
| size | >= 0 |
| position_x/y | 数值范围 [-999999, 999999] |
| tags | JSON 数组，最大 50 个标签 |

## 6. 扩展性设计

### 6.1 自定义标签扩展

未来支持用户添加自定义标签:

```rust
// 添加自定义标签
fn add_custom_tag(node_id: &str, tag: &str) -> Result<(), String> {
    // 验证标签格式
    if tag.len() > 50 {
        return Err("Tag too long".to_string());
    }
    // 添加到标签列表
    // ...
}
```

### 6.2 标签颜色配置

```json
{
  "tag_colors": {
    "JavaScript": "#F7DF1E",
    "Python": "#3776AB",
    "Rust": "#DEA584",
    "测试": "#E34F26",
    "配置": "#1572B6",
    "大文件": "#FF6B6B"
  }
}
```

---

*最后更新: 2026-06-10*
