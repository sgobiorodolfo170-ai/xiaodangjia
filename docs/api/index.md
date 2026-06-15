---
title: API 接口文档
description: 小当家项目的 API 接口参考文档
---

# API 接口文档

## 1. 前端 API (services/api.ts)

### 1.1 项目管理

| 函数 | 说明 | 参数 |
|------|------|------|
| `createProject(name, rootPath)` | 创建项目 | `name: string`, `rootPath: string` |
| `listProjects()` | 获取项目列表 | - |
| `getProject(id)` | 获取项目详情 | `id: string` |
| `deleteProject(id)` | 删除项目 | `id: string` |

### 1.2 文件操作

| 函数 | 说明 | 参数 |
|------|------|------|
| `scanDirectory(projectId, path)` | 扫描目录 | `projectId: string`, `path: string` |
| `readFileContent(path)` | 读取文件内容 | `path: string` |
| `writeFileContent(path, content)` | 写入文件 | `path: string`, `content: string` |
| `deleteFile(path)` | 删除文件 | `path: string` |
| `renameFile(oldPath, newPath)` | 重命名文件 | `oldPath: string`, `newPath: string` |

### 1.3 节点管理

| 函数 | 说明 | 参数 |
|------|------|------|
| `updateNodePosition(id, x, y)` | 更新节点位置 | `id: string`, `x: number`, `y: number` |
| `startFileWatcher(projectId, path)` | 启动文件监听 | `projectId: string`, `path: string` |

### 1.4 AI 分析

| 函数 | 说明 | 参数 |
|------|------|------|
| `analyzeFileRelations(projectId)` | 分析文件关联 | `projectId: string` |
| `generateTags(projectId, fileId)` | 生成文件标签 | `projectId: string`, `fileId: string` |
| `searchFiles(projectId, query)` | 搜索文件 | `projectId: string`, `query: string` |
| `findSimilarFiles(projectId, fileId)` | 查找相似文件 | `projectId: string`, `fileId: string` |

### 1.5 系统交互

| 函数 | 说明 | 参数 |
|------|------|------|
| `openDirectoryDialog()` | 打开目录选择对话框 | - |

---

## 2. Rust 命令 (lib.rs)

### 2.1 项目管理命令

```rust
// 创建项目
#[tauri::command]
fn create_project(name: String, root_path: String, state: State<AppState>) -> Result<Project, String>

// 列出项目
#[tauri::command]
fn list_projects(state: State<AppState>) -> Result<Vec<Project>, String>

// 获取项目
#[tauri::command]
fn get_project(id: String, state: State<AppState>) -> Result<Option<Project>, String>

// 删除项目
#[tauri::command]
fn delete_project(id: String, state: State<AppState>) -> Result<(), String>
```

### 2.2 文件操作命令

```rust
// 扫描目录
#[tauri::command]
fn scan_directory(project_id: String, path: String, state: State<AppState>) -> Result<Vec<FileNode>, String>

// 读取文件内容
#[tauri::command]
fn read_file_content(path: String) -> Result<FileContent, String>

// 写入文件内容
#[tauri::command]
fn write_file_content(path: String, content: String) -> Result<(), String>

// 删除文件
#[tauri::command]
fn delete_file(path: String) -> Result<(), String>

// 重命名文件
#[tauri::command]
fn rename_file(old_path: String, new_path: String) -> Result<(), String>
```

### 2.3 节点管理命令

```rust
// 更新节点位置
#[tauri::command]
fn update_node_position(id: String, x: f64, y: f64, state: State<AppState>) -> Result<(), String>
```

### 2.4 AI 分析命令

```rust
// 分析文件关联
#[tauri::command]
fn analyze_file_relations(project_id: String, state: State<AppState>) -> Result<Vec<FileRelation>, String>

// 生成标签
#[tauri::command]
fn generate_tags(project_id: String, file_id: String, state: State<AppState>) -> Result<Vec<String>, String>

// 搜索文件
#[tauri::command]
fn search_files(project_id: String, query: String, state: State<AppState>) -> Result<Vec<FileNode>, String>

// 查找相似文件
#[tauri::command]
fn find_similar_files(project_id: String, file_id: String, state: State<AppState>) -> Result<Vec<FileNode>, String>
```

### 2.5 系统交互命令

```rust
// 启动文件监听
#[tauri::command]
fn start_file_watcher(project_id: String, path: String, app_handle: AppHandle) -> Result<(), String>

// 打开目录对话框
#[tauri::command]
fn open_directory_dialog() -> Result<Option<String>, String>
```

---

## 3. 数据类型定义

### 3.1 TypeScript 类型 (src/types/index.ts)

```typescript
interface Project {
  id: string;
  name: string;
  root_path: string;
  created_at: string;
  updated_at: string;
}

interface FileNode {
  id: string;
  project_id: string;
  path: string;
  name: string;
  extension: string;
  size: number;
  created_at: string | null;
  modified_at: string | null;
  tags: string[];
  parent_id: string | null;
  position_x: number;
  position_y: number;
  is_collapsed: boolean;
  is_directory: boolean;
}

interface FileContent {
  path: string;
  content: string;
  encoding: string;
  size: number;
}

interface FileRelation {
  id: string;
  project_id: string;
  source_id: string;
  target_id: string;
  relation_type: string;
  confidence: number;
}
```

### 3.2 Rust 类型 (src-tauri/src/lib.rs)

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct FileContent {
    path: String,
    content: String,
    encoding: String,
    size: u64,
}
```

---

## 4. 错误处理

所有 API 返回 `Result<T, String>` 类型，错误信息以字符串形式返回。

常见错误码:

| 错误信息 | 说明 |
|---------|------|
| `Path does not exist` | 指定路径不存在 |
| `File does not exist` | 文件不存在 |
| `Cannot read directory` | 无法读取目录 |
| `Failed to read file: ...` | 文件读取失败 |
| `Failed to write file: ...` | 文件写入失败 |
| `File not found` | 文件未找到 |

---

*最后更新: 2026-06-10*
