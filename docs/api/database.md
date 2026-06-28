---
title: 数据库设计
description: 小当家项目的 SQLite 数据库设计文档
---

# 数据库设计

## 1. 数据库概述

- **数据库类型**: SQLite
- **存储位置**: `{app_data_dir}/xiaodangjia.db`
- **初始化**: 首次启动时自动创建表结构

## 2. 表结构

### 2.1 项目表 (projects)

存储项目基本信息。

```sql
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    root_path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

| 字段 | 类型 | 说明 |
|------|------|------|
| id | TEXT | 主键，UUID |
| name | TEXT | 项目名称 |
| root_path | TEXT | 根目录路径 |
| created_at | TEXT | 创建时间 (ISO 8601) |
| updated_at | TEXT | 更新时间 (ISO 8601) |

### 2.2 文件节点表 (file_nodes)

存储文件/目录元数据及画布位置信息。

```sql
CREATE TABLE file_nodes (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    path TEXT NOT NULL,
    name TEXT NOT NULL,
    extension TEXT,
    size INTEGER DEFAULT 0,
    created_at TEXT,
    modified_at TEXT,
    tags TEXT,
    parent_id TEXT,
    position_x REAL DEFAULT 0,
    position_y REAL DEFAULT 0,
    is_collapsed INTEGER DEFAULT 0,
    is_directory INTEGER DEFAULT 0,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);
```

| 字段 | 类型 | 说明 |
|------|------|------|
| id | TEXT | 主键，UUID |
| project_id | TEXT | 所属项目 ID |
| path | TEXT | 文件完整路径 |
| name | TEXT | 文件名称 |
| extension | TEXT | 文件扩展名 |
| size | INTEGER | 文件大小 (字节) |
| created_at | TEXT | 创建时间 |
| modified_at | TEXT | 修改时间 |
| tags | TEXT | 标签 JSON 数组 |
| parent_id | TEXT | 父节点 ID |
| position_x | REAL | 画布 X 坐标 |
| position_y | REAL | 画布 Y 坐标 |
| is_collapsed | INTEGER | 是否折叠 (0/1) |
| is_directory | INTEGER | 是否目录 (0/1) |

> **注意**: `children` 和 `relatedFiles` 字段不在数据库中存储。它们在查询时由 Rust 后端动态计算：
> - `children`: 根据 `parent_id` 关系从 file_nodes 表聚合子节点 ID 列表
> - `relatedFiles`: 从 `file_relations` 表查询关联文件的 ID 列表
>
> 这两个字段通过 Tauri 命令返回给前端，但不持久化到 SQLite。

### 2.3 文件关联表 (file_relations)

存储文件之间的关联关系。

```sql
CREATE TABLE file_relations (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    relation_type TEXT NOT NULL,
    confidence REAL DEFAULT 0,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (source_id) REFERENCES file_nodes(id) ON DELETE CASCADE,
    FOREIGN KEY (target_id) REFERENCES file_nodes(id) ON DELETE CASCADE
);
```

| 字段 | 类型 | 说明 |
|------|------|------|
| id | TEXT | 主键，UUID |
| project_id | TEXT | 所属项目 ID |
| source_id | TEXT | 源文件 ID |
| target_id | TEXT | 目标文件 ID |
| relation_type | TEXT | 关联类型 |
| confidence | REAL | 置信度 (0-1) |

**关联类型说明**:

| 类型 | 说明 |
|------|------|
| import | 导入/依赖关系 |
| reference | 引用关系 |
| similar_name | 命名相似 |
| same_dir | 同目录 |
| same_ext | 同类型 |
| created_together | 同时创建 |

### 2.4 Agent 日志表 (agent_logs)

存储 AI Agent 操作日志。

```sql
CREATE TABLE agent_logs (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    action TEXT NOT NULL,
    result TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);
```

| 字段 | 类型 | 说明 |
|------|------|------|
| id | TEXT | 主键，UUID |
| project_id | TEXT | 所属项目 ID |
| action | TEXT | 操作类型 |
| result | TEXT | 操作结果 |
| created_at | TEXT | 操作时间 |

## 3. 索引设计

```sql
CREATE INDEX idx_file_nodes_project ON file_nodes(project_id);
CREATE INDEX idx_file_nodes_parent ON file_nodes(parent_id);
CREATE INDEX idx_file_relations_project ON file_relations(project_id);
```

## 4. 实体关系图

```
+-----------+       +---------------+       +------------------+
+  projects |------<| file_nodes    |       | file_relations   |
+-----------+       +---------------+       +------------------+
+  id (PK)  |       | id (PK)       |       | id (PK)          |
+  name     |       | project_id(FK)|------<| source_id (FK)   |
+  root_path|       | parent_id(FK) |       | target_id (FK)   |
+  created  |       | position_x    |       | relation_type    |
+  updated  |       | position_y    |       | confidence       |
+-----------+       | tags (JSON)   |       +------------------+
                    +---------------+
                    +---------------+
+-----------+       | agent_logs    |
+           |       +---------------+
+           |       | id (PK)       |
+           |       | project_id(FK)|
+           |       | action        |
+           |       | result        |
+           |       | created_at    |
+           |       +---------------+
```

## 5. 数据操作示例

### 5.1 创建项目

```rust
pub fn create_project(&self, name: &str, root_path: &str) -> SqlResult<Project> {
    let conn = self.conn.lock().unwrap();
    let now = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO projects (id, name, root_path, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, name, root_path, now, now],
    )?;

    Ok(Project { id, name: name.to_string(), root_path: root_path.to_string(), created_at: now.clone(), updated_at: now })
}
```

### 5.2 插入文件节点

```rust
pub fn insert_file_node(&self, node: &FileNode) -> SqlResult<()> {
    let conn = self.conn.lock().unwrap();
    let tags_json = serde_json::to_string(&node.tags).unwrap_or_default();

    conn.execute(
        r#"INSERT INTO file_nodes
           (id, project_id, path, name, extension, size, created_at, modified_at, tags, parent_id, position_x, position_y, is_collapsed, is_directory)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"#,
        params![node.id, node.project_id, node.path, node.name, node.extension, node.size, node.created_at, node.modified_at, tags_json, node.parent_id, node.position_x, node.position_y, node.is_collapsed as i32, node.is_directory as i32],
    )?;
    Ok(())
}
```

### 5.3 更新节点位置

```rust
pub fn update_node_position(&self, id: &str, x: f64, y: f64) -> SqlResult<()> {
    let conn = self.conn.lock().unwrap();
    conn.execute(
        "UPDATE file_nodes SET position_x = ?1, position_y = ?2 WHERE id = ?3",
        params![x, y, id],
    )?;
    Ok(())
}
```

## 6. 迁移策略

未来版本更新时，使用 SQLite 的 ALTER TABLE 进行表结构迁移:

```sql
-- 示例: 添加新字段
ALTER TABLE file_nodes ADD COLUMN description TEXT;
```

---

*最后更新: 2026-06-16*
