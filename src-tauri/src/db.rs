use rusqlite::{Connection, Result as SqlResult, params};
use std::path::PathBuf;
use std::sync::Mutex;
use chrono::Utc;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(app_data_dir: PathBuf) -> SqlResult<Self> {
        std::fs::create_dir_all(&app_data_dir).ok();
        let db_path = app_data_dir.join("xiaodangjia.db");
        let conn = Connection::open(&db_path)?;
        
        let db = Database {
            conn: Mutex::new(conn),
        };
        
        db.init_tables()?;
        Ok(db)
    }
    
    fn init_tables(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        
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
        "#)?;
        
        Ok(())
    }
}

// Project operations
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Database {
    pub fn create_project(&self, name: &str, root_path: &str) -> SqlResult<Project> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, root_path, created_at, updated_at FROM projects ORDER BY updated_at DESC")?;
        
        let projects = stmt.query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                root_path: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        
        Ok(projects)
    }
    
    pub fn get_project(&self, id: &str) -> SqlResult<Option<Project>> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(())
    }
}

// File node operations
#[derive(Debug, Serialize, Deserialize, Clone)]
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
}

impl Database {
    pub fn insert_file_node(&self, node: &FileNode) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
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
            })
        })?.filter_map(|r| r.ok()).collect();
        
        Ok(nodes)
    }
    
    pub fn update_node_position(&self, id: &str, x: f64, y: f64) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE file_nodes SET position_x = ?1, position_y = ?2 WHERE id = ?3",
            params![x, y, id],
        )?;
        Ok(())
    }
    
    pub fn delete_file_nodes_by_project(&self, project_id: &str) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM file_nodes WHERE project_id = ?1", params![project_id])?;
        Ok(())
    }
}
