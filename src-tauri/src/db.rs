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
        })?.filter_map(|r| r.ok()).collect();

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
            })
        })?.filter_map(|r| r.ok()).collect();

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

    pub fn delete_file_nodes_by_project(&self, project_id: &str) -> SqlResult<()> {
        let conn = &self.conn;
        conn.execute("DELETE FROM file_nodes WHERE project_id = ?1", params![project_id])?;
        Ok(())
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
        };

        db.insert_file_node(&node).unwrap();
        db.delete_project(&project.id).unwrap();

        let nodes = db.get_file_nodes_by_project(&project.id).unwrap();
        assert_eq!(nodes.len(), 0);
    }
}
