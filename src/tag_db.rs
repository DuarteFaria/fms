use rusqlite::{Connection, Result, params};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

pub(crate) fn normalize_path(path: &PathBuf) -> String {
    let mut normalized = path.to_string_lossy().to_string();
    if normalized.ends_with('/') && normalized.len() > 1 {
        normalized.pop();
    }
    normalized
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub file_type: FileType,
    pub size: u64,
    pub modified: i64,
    pub parent: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileType {
    File,
    Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub color: Option<String>,
    pub file_count: usize,
}

pub struct TagDatabase {
    pub(crate) conn: Arc<Mutex<Connection>>,
}

impl TagDatabase {
    pub fn new() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS files (
                path TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                file_type TEXT NOT NULL,
                size INTEGER NOT NULL,
                modified INTEGER NOT NULL,
                parent TEXT
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS tags (
                name TEXT PRIMARY KEY,
                color TEXT
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS file_tags (
                file_path TEXT NOT NULL,
                tag_name TEXT NOT NULL,
                PRIMARY KEY (file_path, tag_name),
                FOREIGN KEY (file_path) REFERENCES files(path) ON DELETE CASCADE,
                FOREIGN KEY (tag_name) REFERENCES tags(name) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_files_name ON files(name)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_files_parent ON files(parent)",
            [],
        )?;

        if let Err(e) = conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
                name,
                path,
                content='files',
                content_rowid='rowid'
            )",
            [],
        ) {
            eprintln!("Failed to create files_fts virtual table: {}", e);
        }

        Ok(TagDatabase {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn insert_file(&self, entry: &FileEntry) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO files (path, name, file_type, size, modified, parent)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                normalize_path(&entry.path),
                entry.name,
                match entry.file_type {
                    FileType::File => "file",
                    FileType::Directory => "directory",
                },
                entry.size,
                entry.modified,
                entry.parent.as_ref().map(|p| normalize_path(p))
            ],
        )?;

        let normalized_path = normalize_path(&entry.path);
        conn.execute(
            "INSERT INTO files_fts (rowid, name, path) VALUES (
                (SELECT rowid FROM files WHERE path = ?1),
                ?2,
                ?3
            ) ON CONFLICT(rowid) DO UPDATE SET name = ?2, path = ?3",
            params![
                normalized_path,
                entry.name,
                normalized_path
            ],
        )?;

        Ok(())
    }

    pub fn add_tag_to_file(&self, file_path: &PathBuf, tag_name: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        
        conn.execute(
            "INSERT OR IGNORE INTO tags (name) VALUES (?1)",
            params![tag_name],
        )?;

        conn.execute(
            "INSERT OR IGNORE INTO file_tags (file_path, tag_name) VALUES (?1, ?2)",
            params![normalize_path(file_path), tag_name],
        )?;

        Ok(())
    }

    pub fn get_all_tags(&self) -> Result<Vec<Tag>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT t.name, t.color, COUNT(ft.file_path) as file_count
             FROM tags t
             LEFT JOIN file_tags ft ON t.name = ft.tag_name
             GROUP BY t.name, t.color
             ORDER BY t.name"
        )?;

        let tags = stmt.query_map([], |row| {
            Ok(Tag {
                name: row.get(0)?,
                color: row.get(1)?,
                file_count: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(tags)
    }

    pub fn get_files_by_tag(&self, tag_name: &str) -> Result<Vec<FileEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT f.path, f.name, f.file_type, f.size, f.modified, f.parent
             FROM files f
             INNER JOIN file_tags ft ON f.path = ft.file_path
             WHERE ft.tag_name = ?1
             ORDER BY f.name"
        )?;

        let files = stmt.query_map(params![tag_name], |row| {
            Ok(FileEntry {
                path: PathBuf::from(row.get::<_, String>(0)?),
                name: row.get(1)?,
                file_type: match row.get::<_, String>(2)?.as_str() {
                    "file" => FileType::File,
                    "directory" => FileType::Directory,
                    _ => FileType::File,
                },
                size: row.get(3)?,
                modified: row.get(4)?,
                parent: row.get::<_, Option<String>>(5)?.map(PathBuf::from),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(files)
    }

    pub fn get_files_in_directory(&self, dir_path: &PathBuf) -> Result<Vec<FileEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT path, name, file_type, size, modified, parent
             FROM files
             WHERE parent = ?1
             ORDER BY file_type DESC, name"
        )?;

        let files = stmt.query_map(params![normalize_path(dir_path)], |row| {
            Ok(FileEntry {
                path: PathBuf::from(row.get::<_, String>(0)?),
                name: row.get(1)?,
                file_type: match row.get::<_, String>(2)?.as_str() {
                    "file" => FileType::File,
                    "directory" => FileType::Directory,
                    _ => FileType::File,
                },
                size: row.get(3)?,
                modified: row.get(4)?,
                parent: row.get::<_, Option<String>>(5)?.map(PathBuf::from),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(files)
    }

    pub fn get_directory(&self, dir_path: &PathBuf) -> Result<Option<FileEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT path, name, file_type, size, modified, parent
             FROM files
             WHERE path = ?1 AND file_type = 'directory'"
        )?;

        let mut entries = stmt.query_map(params![normalize_path(dir_path)], |row| {
            Ok(FileEntry {
                path: PathBuf::from(row.get::<_, String>(0)?),
                name: row.get(1)?,
                file_type: FileType::Directory,
                size: row.get(3)?,
                modified: row.get(4)?,
                parent: row.get::<_, Option<String>>(5)?.map(PathBuf::from),
            })
        })?;

        if let Some(entry) = entries.next() {
            Ok(Some(entry?))
        } else {
            Ok(None)
        }
    }
}
