use rusqlite::{Connection, Result as SqlResult, params};
use std::path::PathBuf;
use chrono::Utc;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

pub struct Database {
    pub conn: Connection,
}

impl Database {
    pub fn new(app_data_dir: PathBuf) -> SqlResult<Self> {
        std::fs::create_dir_all(&app_data_dir).ok();
        let db_path = app_data_dir.join("xiaodangjia.db");
        let conn = Connection::open(&db_path)?;
        let db = Database { conn };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> SqlResult<()> {
        let conn = &self.conn;

        // Enable foreign keys but defer constraints
        conn.execute_batch(r#"
            PRAGMA foreign_keys = ON;
            PRAGMA defer_foreign_keys = ON;
        "#)?;

        // Schema version tracking for safe migrations
        conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL
            );
        "#)?;

        let current_version: i32 = conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )?;

        conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                root_path TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS file_nodes (
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

            CREATE TABLE IF NOT EXISTS file_relations (
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

            CREATE TABLE IF NOT EXISTS agent_logs (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                action TEXT NOT NULL,
                result TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_file_nodes_project ON file_nodes(project_id);
            CREATE INDEX IF NOT EXISTS idx_file_nodes_parent ON file_nodes(parent_id);
            CREATE INDEX IF NOT EXISTS idx_file_relations_project ON file_relations(project_id);
            
            -- Favorites table
            CREATE TABLE IF NOT EXISTS favorites (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                file_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
                FOREIGN KEY (file_id) REFERENCES file_nodes(id) ON DELETE CASCADE,
                UNIQUE(project_id, file_id)
            );
            
            -- Tags table
            CREATE TABLE IF NOT EXISTS tags (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                color TEXT DEFAULT '#3b82f6'
            );
            
            -- File tags junction table
            CREATE TABLE IF NOT EXISTS file_tags (
                file_id TEXT NOT NULL,
                tag_id TEXT NOT NULL,
                PRIMARY KEY (file_id, tag_id),
                FOREIGN KEY (file_id) REFERENCES file_nodes(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            );
            
            CREATE INDEX IF NOT EXISTS idx_favorites_project ON favorites(project_id);
            CREATE INDEX IF NOT EXISTS idx_file_tags_file ON file_tags(file_id);
            CREATE INDEX IF NOT EXISTS idx_file_tags_tag ON file_tags(tag_id);
            
            -- File edit history table
            CREATE TABLE IF NOT EXISTS file_edit_history (
                id TEXT PRIMARY KEY,
                file_id TEXT NOT NULL,
                file_path TEXT NOT NULL,
                content TEXT NOT NULL,
                diff TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY (file_id) REFERENCES file_nodes(id) ON DELETE CASCADE
            );
            
            CREATE INDEX IF NOT EXISTS idx_file_edit_history_file ON file_edit_history(file_id);

            -- Cached file embeddings for similarity search (P1-11).
            -- Avoids re-reading 500 files and rebuilding TF-IDF/embedding on every query.
            CREATE TABLE IF NOT EXISTS file_embeddings (
                file_id TEXT NOT NULL PRIMARY KEY,
                embedding TEXT NOT NULL,  -- JSON-encoded Vec<f64>
                updated_at TEXT NOT NULL,
                FOREIGN KEY (file_id) REFERENCES file_nodes(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_file_embeddings_file ON file_embeddings(file_id);
        "#)?;

        // Apply incremental migrations if the schema is behind.
        // Each migration runs in its own transaction.
        let migrations: Vec<(i32, &str)> = vec![
            // v1: file_embeddings table (added as part of P1-11).
            // This is idempotent because CREATE TABLE IF NOT EXISTS is used above,
            // but adding the migration entry ensures the version is tracked.
        ];

        for (version, sql) in &migrations {
            if *version > current_version {
                conn.execute_batch(sql)?;
                conn.execute(
                    "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
                    params![version, chrono::Utc::now().to_rfc3339()],
                )?;
                log::info!("Applied DB migration v{}", version);
            }
        }

        // If the schema was at version 0 (fresh install), mark as current.
        if current_version == 0 {
            conn.execute(
                "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
                params![1, chrono::Utc::now().to_rfc3339()],
            )?;
        }

        Ok(())
    }
}

// Project operations
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Database {
    pub fn create_project(&self, name: &str, root_path: &str) -> SqlResult<Project> {
        let conn = &self.conn;
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4().to_string();

        conn.execute(
            "INSERT INTO projects (id, name, root_path, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, name, root_path, now, now],
        )?;

        Ok(Project {
            id,
            name: name.to_string(),
            root_path: root_path.to_string(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn list_projects(&self) -> SqlResult<Vec<Project>> {
        let conn = &self.conn;
        let mut stmt = conn.prepare("SELECT id, name, root_path, created_at, updated_at FROM projects ORDER BY updated_at DESC")?;

        let projects = stmt.query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                root_path: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?.filter_map(|r| match r { Ok(v) => Some(v), Err(e) => { log::warn!("DB query row skipped: {}", e); None } }).collect();

        Ok(projects)
    }

    pub fn get_project(&self, id: &str) -> SqlResult<Option<Project>> {
        let conn = &self.conn;
        let mut stmt = conn.prepare("SELECT id, name, root_path, created_at, updated_at FROM projects WHERE id = ?1")?;

        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                root_path: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn delete_project(&self, id: &str) -> SqlResult<()> {
        let conn = &self.conn;
        conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Get project root path for path validation
    pub fn get_project_root(&self, project_id: &str) -> SqlResult<Option<String>> {
        let conn = &self.conn;
        let mut stmt = conn.prepare("SELECT root_path FROM projects WHERE id = ?1")?;
        let mut rows = stmt.query(params![project_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }
    
    // ============= Favorites =============
    pub fn add_favorite(&self, project_id: &str, file_id: &str) -> SqlResult<()> {
        let conn = &self.conn;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR IGNORE INTO favorites (id, project_id, file_id, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, project_id, file_id, now],
        )?;
        Ok(())
    }
    
    pub fn remove_favorite(&self, project_id: &str, file_id: &str) -> SqlResult<()> {
        let conn = &self.conn;
        conn.execute(
            "DELETE FROM favorites WHERE project_id = ?1 AND file_id = ?2",
            params![project_id, file_id],
        )?;
        Ok(())
    }
    
    pub fn get_favorites(&self, project_id: &str) -> SqlResult<Vec<String>> {
        let conn = &self.conn;
        let mut stmt = conn.prepare("SELECT file_id FROM favorites WHERE project_id = ?1")?;
        let ids = stmt.query_map(params![project_id], |row| row.get(0))?
            .filter_map(|r| match r { Ok(v) => Some(v), Err(e) => { log::warn!("DB query row skipped: {}", e); None } })
            .collect();
        Ok(ids)
    }

    // ============= File Embeddings =============

    /// Store or update an embedding for a file.
    pub fn upsert_file_embedding(&self, file_id: &str, embedding: &[f64]) -> SqlResult<()> {
        let emb_json = serde_json::to_string(embedding).unwrap_or_default();
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR REPLACE INTO file_embeddings (file_id, embedding, updated_at) VALUES (?1, ?2, ?3)",
            params![file_id, emb_json, now],
        )?;
        Ok(())
    }

    /// Retrieve the embedding for a file, if cached.
    pub fn get_file_embedding(&self, file_id: &str) -> SqlResult<Option<Vec<f64>>> {
        let mut stmt = self.conn.prepare(
            "SELECT embedding FROM file_embeddings WHERE file_id = ?1"
        )?;
        let mut rows = stmt.query(params![file_id])?;
        if let Some(row) = rows.next()? {
            let emb_str: String = row.get(0)?;
            let embedding: Vec<f64> = serde_json::from_str(&emb_str).unwrap_or_default();
            Ok(Some(embedding))
        } else {
            Ok(None)
        }
    }

    /// Retrieve all cached embeddings for files in a project.
    pub fn get_project_embeddings(&self, project_id: &str) -> SqlResult<Vec<(String, Vec<f64>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT e.file_id, e.embedding FROM file_embeddings e
             INNER JOIN file_nodes f ON e.file_id = f.id
             WHERE f.project_id = ?1"
        )?;
        let results = stmt.query_map(params![project_id], |row| {
            let file_id: String = row.get(0)?;
            let emb_str: String = row.get(1)?;
            Ok((file_id, emb_str))
        })?.filter_map(|r| match r {
            Ok((id, emb_str)) => {
                let embedding: Vec<f64> = serde_json::from_str(&emb_str).unwrap_or_default();
                Some((id, embedding))
            }
            Err(e) => {
                log::warn!("DB query row skipped: {}", e);
                None
            }
        }).collect();
        Ok(results)
    }

    /// Delete cached embeddings for files that no longer exist in file_nodes.
    pub fn prune_stale_embeddings(&self, project_id: &str) -> SqlResult<usize> {
        Ok(self.conn.execute(
            "DELETE FROM file_embeddings WHERE file_id NOT IN (SELECT id FROM file_nodes WHERE project_id = ?1)",
            params![project_id],
        )?)
    }

    /// Check if a file is favorited ? uses SELECT EXISTS for O(1) lookup (P3-2).
    pub fn is_favorite(&self, project_id: &str, file_id: &str) -> SqlResult<bool> {
        let conn = &self.conn;
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM favorites WHERE project_id = ?1 AND file_id = ?2)",
            params![project_id, file_id],
            |row| row.get(0),
        )?;
        Ok(exists)
    }
    
    // ============= Tags =============
    pub fn create_tag(&self, name: &str, color: &str) -> SqlResult<Tag> {
        let conn = &self.conn;
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO tags (id, name, color) VALUES (?1, ?2, ?3)",
            params![id, name, color],
        )?;
        Ok(Tag { id, name: name.to_string(), color: color.to_string() })
    }
    
    pub fn list_tags(&self) -> SqlResult<Vec<Tag>> {
        let conn = &self.conn;
        let mut stmt = conn.prepare("SELECT id, name, color FROM tags ORDER BY name")?;
        let tags = stmt.query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
            })
        })?.filter_map(|r| match r { Ok(v) => Some(v), Err(e) => { log::warn!("DB query row skipped: {}", e); None } }).collect();
        Ok(tags)
    }
    
    pub fn delete_tag(&self, id: &str) -> SqlResult<()> {
        let conn = &self.conn;
        conn.execute("DELETE FROM tags WHERE id = ?1", params![id])?;
        Ok(())
    }
    
    pub fn add_file_tag(&self, file_id: &str, tag_id: &str) -> SqlResult<()> {
        let conn = &self.conn;
        conn.execute(
            "INSERT OR IGNORE INTO file_tags (file_id, tag_id) VALUES (?1, ?2)",
            params![file_id, tag_id],
        )?;
        Ok(())
    }
    
    pub fn remove_file_tag(&self, file_id: &str, tag_id: &str) -> SqlResult<()> {
        let conn = &self.conn;
        conn.execute(
            "DELETE FROM file_tags WHERE file_id = ?1 AND tag_id = ?2",
            params![file_id, tag_id],
        )?;
        Ok(())
    }
    
    pub fn get_file_tags(&self, file_id: &str) -> SqlResult<Vec<Tag>> {
        let conn = &self.conn;
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, t.color FROM tags t 
             INNER JOIN file_tags ft ON t.id = ft.tag_id 
             WHERE ft.file_id = ?1"
        )?;
        let tags = stmt.query_map(params![file_id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
            })
        })?.filter_map(|r| match r { Ok(v) => Some(v), Err(e) => { log::warn!("DB query row skipped: {}", e); None } }).collect();
        Ok(tags)
    }
    
    // File edit history methods
    pub fn add_file_edit_history(&self, file_id: &str, file_path: &str, content: &str, diff: Option<&str>) -> SqlResult<FileEditHistory> {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        
        self.conn.execute(
            "INSERT INTO file_edit_history (id, file_id, file_path, content, diff, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, file_id, file_path, content, diff, created_at]
        )?;
        
        Ok(FileEditHistory {
            id,
            file_id: file_id.to_string(),
            file_path: file_path.to_string(),
            content: content.to_string(),
            diff: diff.map(|s| s.to_string()),
            created_at,
        })
    }
    
    pub fn get_file_edit_history(&self, file_id: &str) -> SqlResult<Vec<FileEditHistory>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, file_id, file_path, content, diff, created_at FROM file_edit_history 
             WHERE file_id = ?1 ORDER BY created_at DESC"
        )?;
        
        let histories = stmt.query_map(params![file_id], |row| {
            Ok(FileEditHistory {
                id: row.get(0)?,
                file_id: row.get(1)?,
                file_path: row.get(2)?,
                content: row.get(3)?,
                diff: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?.filter_map(|r| match r { Ok(v) => Some(v), Err(e) => { log::warn!("DB query row skipped: {}", e); None } }).collect();
        
        Ok(histories)
    }
    
    pub fn get_file_edit_history_by_id(&self, id: &str) -> SqlResult<Option<FileEditHistory>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, file_id, file_path, content, diff, created_at FROM file_edit_history WHERE id = ?1"
        )?;
        
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(FileEditHistory {
                id: row.get(0)?,
                file_id: row.get(1)?,
                file_path: row.get(2)?,
                content: row.get(3)?,
                diff: row.get(4)?,
                created_at: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }
    
    pub fn delete_file_edit_history(&self, id: &str) -> SqlResult<()> {
        self.conn.execute("DELETE FROM file_edit_history WHERE id = ?1", params![id])?;
        Ok(())
    }
}

// File edit history type
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileEditHistory {
    pub id: String,
    pub file_id: String,
    pub file_path: String,
    pub content: String,
    pub diff: Option<String>,
    pub created_at: String,
}

// Tag type
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: String,
}

// File relation type
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileRelation {
    pub id: String,
    pub project_id: String,
    pub source_id: String,
    pub target_id: String,
    pub relation_type: String,
    pub confidence: f64,
}

// File node operations
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileNode {
    pub id: String,
    pub project_id: String,
    pub path: String,
    pub name: String,
    pub extension: String,
    pub size: i64,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub tags: Vec<String>,
    pub parent_id: Option<String>,
    pub position_x: f64,
    pub position_y: f64,
    pub is_collapsed: bool,
    pub is_directory: bool,
    #[serde(default)]
    pub children: Vec<String>,
    #[serde(default)]
    pub related_files: Vec<String>,
}

impl Database {
    pub fn insert_file_node(&self, node: &FileNode) -> SqlResult<()> {
        let conn = &self.conn;
        let tags_json = serde_json::to_string(&node.tags).unwrap_or_default();

        conn.execute(
            r#"INSERT INTO file_nodes
               (id, project_id, path, name, extension, size, created_at, modified_at, tags, parent_id, position_x, position_y, is_collapsed, is_directory)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"#,
            params![
                node.id,
                node.project_id,
                node.path,
                node.name,
                node.extension,
                node.size,
                node.created_at,
                node.modified_at,
                tags_json,
                node.parent_id,
                node.position_x,
                node.position_y,
                node.is_collapsed as i32,
                node.is_directory as i32,
            ],
        )?;
        Ok(())
    }

    pub fn get_file_nodes_by_project(&self, project_id: &str) -> SqlResult<Vec<FileNode>> {
        let conn = &self.conn;
        let mut stmt = conn.prepare(
            "SELECT id, project_id, path, name, extension, size, created_at, modified_at, tags, parent_id, position_x, position_y, is_collapsed, is_directory
             FROM file_nodes WHERE project_id = ?1"
        )?;

        let nodes = stmt.query_map(params![project_id], |row| {
            let tags_str: String = row.get(8)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();

            Ok(FileNode {
                id: row.get(0)?,
                project_id: row.get(1)?,
                path: row.get(2)?,
                name: row.get(3)?,
                extension: row.get(4)?,
                size: row.get(5)?,
                created_at: row.get(6)?,
                modified_at: row.get(7)?,
                tags,
                parent_id: row.get(9)?,
                position_x: row.get(10)?,
                position_y: row.get(11)?,
                is_collapsed: row.get::<_, i32>(12)? != 0,
                is_directory: row.get::<_, i32>(13)? != 0,
                children: Vec::new(),
                related_files: Vec::new(),
            })
        })?.filter_map(|r| match r { Ok(v) => Some(v), Err(e) => { log::warn!("DB query row skipped: {}", e); None } }).collect();

        Ok(nodes)
    }

    pub fn update_node_position(&self, id: &str, x: f64, y: f64) -> SqlResult<()> {
        let conn = &self.conn;
        conn.execute(
            "UPDATE file_nodes SET position_x = ?1, position_y = ?2 WHERE id = ?3",
            params![x, y, id],
        )?;
        Ok(())
    }

    /// Get a single file node by ID (avoids loading all nodes for the project).
    pub fn get_file_node_by_id(&self, id: &str) -> SqlResult<Option<FileNode>> {
        let conn = &self.conn;
        let mut stmt = conn.prepare(
            "SELECT id, project_id, path, name, extension, size, created_at, modified_at, tags, parent_id, position_x, position_y, is_collapsed, is_directory
             FROM file_nodes WHERE id = ?1"
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let tags_str: String = row.get(8)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            Ok(Some(FileNode {
                id: row.get(0)?,
                project_id: row.get(1)?,
                path: row.get(2)?,
                name: row.get(3)?,
                extension: row.get(4)?,
                size: row.get(5)?,
                created_at: row.get(6)?,
                modified_at: row.get(7)?,
                tags,
                parent_id: row.get(9)?,
                position_x: row.get(10)?,
                position_y: row.get(11)?,
                is_collapsed: row.get::<_, i32>(12)? != 0,
                is_directory: row.get::<_, i32>(13)? != 0,
                children: Vec::new(),
                related_files: Vec::new(),
            }))
        } else {
            Ok(None)
        }
    }

    #[allow(dead_code)]
    pub fn delete_file_nodes_by_project(&self, project_id: &str) -> SqlResult<()> {
        let conn = &self.conn;
        conn.execute("DELETE FROM file_nodes WHERE project_id = ?1", params![project_id])?;
        Ok(())
    }

    pub fn delete_file_node_by_path(&self, path: &str) -> SqlResult<()> {
        let conn = &self.conn;
        conn.execute("DELETE FROM file_nodes WHERE path = ?1", params![path])?;
        Ok(())
    }

    pub fn update_file_node_path(&self, old_path: &str, new_path: &str, new_name: &str, new_ext: &str) -> SqlResult<()> {
        let conn = &self.conn;
        conn.execute(
            "UPDATE file_nodes SET path = ?1, name = ?2, extension = ?3 WHERE path = ?4",
            params![new_path, new_name, new_ext, old_path],
        )?;
        Ok(())
    }

    pub fn list_tags_by_project(&self, project_id: &str) -> SqlResult<Vec<Tag>> {
        let conn = &self.conn;
        let mut stmt = conn.prepare(
            "SELECT DISTINCT t.id, t.name, t.color FROM tags t
             INNER JOIN file_tags ft ON ft.tag_id = t.id
             INNER JOIN file_nodes fn ON fn.id = ft.file_id
             WHERE fn.project_id = ?1
             ORDER BY t.name",
        )?;
        let tags = stmt.query_map(params![project_id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
            })
        })?.filter_map(|r| match r { Ok(v) => Some(v), Err(e) => { log::warn!("DB query row skipped: {}", e); None } }).collect();
        Ok(tags)
    }

    /// Atomically replace all file nodes for a project within a single transaction.
    /// This avoids the half-empty state that occurs when delete and insert are separate.
    pub fn replace_file_nodes(&self, project_id: &str, nodes: &[FileNode]) -> SqlResult<()> {
        let tx = self.conn.unchecked_transaction()?;
        {
            tx.execute("DELETE FROM file_nodes WHERE project_id = ?1", params![project_id])?;
            for node in nodes {
                let tags_json = serde_json::to_string(&node.tags).unwrap_or_default();
                tx.execute(
                    r#"INSERT INTO file_nodes
                       (id, project_id, path, name, extension, size, created_at, modified_at, tags, parent_id, position_x, position_y, is_collapsed, is_directory)
                       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"#,
                    params![
                        node.id,
                        node.project_id,
                        node.path,
                        node.name,
                        node.extension,
                        node.size,
                        node.created_at,
                        node.modified_at,
                        tags_json,
                        node.parent_id,
                        node.position_x,
                        node.position_y,
                        node.is_collapsed as i32,
                        node.is_directory as i32,
                    ],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Batch insert file nodes within a transaction for better performance
    #[allow(dead_code)]
    pub fn insert_file_nodes_batch(&self, nodes: &[FileNode]) -> SqlResult<()> {
        let tx = self.conn.unchecked_transaction()?;
        {
            for node in nodes {
                let tags_json = serde_json::to_string(&node.tags).unwrap_or_default();
                tx.execute(
                    r#"INSERT INTO file_nodes
                       (id, project_id, path, name, extension, size, created_at, modified_at, tags, parent_id, position_x, position_y, is_collapsed, is_directory)
                       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"#,
                    params![
                        node.id,
                        node.project_id,
                        node.path,
                        node.name,
                        node.extension,
                        node.size,
                        node.created_at,
                        node.modified_at,
                        tags_json,
                        node.parent_id,
                        node.position_x,
                        node.position_y,
                        node.is_collapsed as i32,
                        node.is_directory as i32,
                    ],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_relations_by_project(&self, project_id: &str) -> SqlResult<Vec<FileRelation>> {
        let conn = &self.conn;
        let mut stmt = conn.prepare(
            "SELECT id, project_id, source_id, target_id, relation_type, confidence
             FROM file_relations WHERE project_id = ?1"
        )?;

        let relations = stmt.query_map(params![project_id], |row| {
            Ok(FileRelation {
                id: row.get(0)?,
                project_id: row.get(1)?,
                source_id: row.get(2)?,
                target_id: row.get(3)?,
                relation_type: row.get(4)?,
                confidence: row.get(5)?,
            })
        })?.filter_map(|r| match r { Ok(v) => Some(v), Err(e) => { log::warn!("DB query row skipped: {}", e); None } }).collect();

        Ok(relations)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_db() -> Database {
        let dir = std::env::temp_dir().join(format!("xiaodangjia_test_{}_{}", std::process::id(), uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).ok();
        Database::new(dir).expect("Failed to create test DB")
    }

    #[test]
    fn test_create_project() {
        let db = create_test_db();
        let project = db.create_project("test", "/tmp/test").unwrap();
        assert_eq!(project.name, "test");
        assert_eq!(project.root_path, "/tmp/test");
        assert!(!project.id.is_empty());
    }

    #[test]
    fn test_list_projects() {
        let db = create_test_db();
        db.create_project("p1", "/tmp/p1").unwrap();
        db.create_project("p2", "/tmp/p2").unwrap();
        let projects = db.list_projects().unwrap();
        assert_eq!(projects.len(), 2);
    }

    #[test]
    fn test_get_project() {
        let db = create_test_db();
        let created = db.create_project("test", "/tmp/test").unwrap();
        let fetched = db.get_project(&created.id).unwrap().expect("Project not found");
        assert_eq!(fetched.name, "test");
    }

    #[test]
    fn test_get_project_not_found() {
        let db = create_test_db();
        let result = db.get_project("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_delete_project() {
        let db = create_test_db();
        let created = db.create_project("test", "/tmp/test").unwrap();
        db.delete_project(&created.id).unwrap();
        let projects = db.list_projects().unwrap();
        assert_eq!(projects.len(), 0);
    }

    #[test]
    fn test_insert_and_get_file_nodes() {
        let db = create_test_db();
        let project = db.create_project("test", "/tmp/test").unwrap();

        let node = FileNode {
            id: "node-1".to_string(),
            project_id: project.id.clone(),
            path: "/tmp/test/main.ts".to_string(),
            name: "main.ts".to_string(),
            extension: "ts".to_string(),
            size: 1024,
            created_at: None,
            modified_at: None,
            tags: vec!["TypeScript".to_string()],
            parent_id: None,
            position_x: 0.0,
            position_y: 0.0,
            is_collapsed: false,
            is_directory: false,
            children: Vec::new(),
            related_files: Vec::new(),
        };

        db.insert_file_node(&node).unwrap();
        let nodes = db.get_file_nodes_by_project(&project.id).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].name, "main.ts");
        assert_eq!(nodes[0].tags, vec!["TypeScript"]);
    }

    #[test]
    fn test_update_node_position() {
        let db = create_test_db();
        let project = db.create_project("test", "/tmp/test").unwrap();

        let node = FileNode {
            id: "node-1".to_string(),
            project_id: project.id.clone(),
            path: "/tmp/test/main.ts".to_string(),
            name: "main.ts".to_string(),
            extension: "ts".to_string(),
            size: 1024,
            created_at: None,
            modified_at: None,
            tags: vec![],
            parent_id: None,
            position_x: 0.0,
            position_y: 0.0,
            is_collapsed: false,
            is_directory: false,
            children: Vec::new(),
            related_files: Vec::new(),
        };

        db.insert_file_node(&node).unwrap();
        db.update_node_position("node-1", 100.0, 200.0).unwrap();

        let nodes = db.get_file_nodes_by_project(&project.id).unwrap();
        assert!((nodes[0].position_x - 100.0).abs() < 0.001);
        assert!((nodes[0].position_y - 200.0).abs() < 0.001);
    }

    #[test]
    fn test_delete_file_nodes_by_project() {
        let db = create_test_db();
        let project = db.create_project("test", "/tmp/test").unwrap();

        let node = FileNode {
            id: "node-1".to_string(),
            project_id: project.id.clone(),
            path: "/tmp/test/main.ts".to_string(),
            name: "main.ts".to_string(),
            extension: "ts".to_string(),
            size: 1024,
            created_at: None,
            modified_at: None,
            tags: vec![],
            parent_id: None,
            position_x: 0.0,
            position_y: 0.0,
            is_collapsed: false,
            is_directory: false,
            children: Vec::new(),
            related_files: Vec::new(),
        };

        db.insert_file_node(&node).unwrap();
        db.delete_file_nodes_by_project(&project.id).unwrap();

        let nodes = db.get_file_nodes_by_project(&project.id).unwrap();
        assert_eq!(nodes.len(), 0);
    }

    #[test]
    fn test_get_project_root() {
        let db = create_test_db();
        let project = db.create_project("test", "/tmp/test").unwrap();
        let root = db.get_project_root(&project.id).unwrap().expect("Root not found");
        assert_eq!(root, "/tmp/test");
    }

    #[test]
    fn test_cascade_delete_project_removes_nodes() {
        let db = create_test_db();
        let project = db.create_project("test", "/tmp/test").unwrap();

        let node = FileNode {
            id: "node-1".to_string(),
            project_id: project.id.clone(),
            path: "/tmp/test/main.ts".to_string(),
            name: "main.ts".to_string(),
            extension: "ts".to_string(),
            size: 1024,
            created_at: None,
            modified_at: None,
            tags: vec![],
            parent_id: None,
            position_x: 0.0,
            position_y: 0.0,
            is_collapsed: false,
            is_directory: false,
            children: Vec::new(),
            related_files: Vec::new(),
        };

        db.insert_file_node(&node).unwrap();
        db.delete_project(&project.id).unwrap();

        let nodes = db.get_file_nodes_by_project(&project.id).unwrap();
        assert_eq!(nodes.len(), 0);
    }
}
