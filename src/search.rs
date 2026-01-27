use std::path::PathBuf;
use std::sync::Arc;
use rusqlite::{Result, params};

use crate::tag_db::{TagDatabase, FileEntry, normalize_path};

pub struct SearchEngine {
    pub(crate) tag_db: Arc<TagDatabase>,
}

impl SearchEngine {
    pub fn new(tag_db: Arc<TagDatabase>) -> Self {
        SearchEngine { tag_db }
    }

    pub fn search(&self, query: &str) -> Result<Vec<FileEntry>> {
        if query.is_empty() {
            return Ok(vec![]);
        }

        let conn = self.tag_db.conn.lock().unwrap();
        let search_pattern = format!("%{}%", query);

        let mut stmt = conn.prepare(
            "SELECT DISTINCT path, name, file_type, size, modified, parent
             FROM files
             WHERE LOWER(name) LIKE LOWER(?1) OR LOWER(path) LIKE LOWER(?1)
             ORDER BY name
             LIMIT 1000"
        )?;

        let files = stmt.query_map(params![search_pattern], |row| {
            Ok(FileEntry {
                path: PathBuf::from(row.get::<_, String>(0)?),
                name: row.get(1)?,
                file_type: match row.get::<_, String>(2)?.as_str() {
                    "file" => crate::tag_db::FileType::File,
                    "directory" => crate::tag_db::FileType::Directory,
                    _ => crate::tag_db::FileType::File,
                },
                size: row.get(3)?,
                modified: row.get(4)?,
                parent: row.get::<_, Option<String>>(5)?.map(PathBuf::from),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(files)
    }

    pub fn search_in_directory(&self, dir_path: &PathBuf, query: &str) -> Result<Vec<FileEntry>> {
        if query.is_empty() {
            return self.tag_db.get_files_in_directory(dir_path);
        }

        let conn = self.tag_db.conn.lock().unwrap();
        let search_pattern = format!("%{}%", query);

        let mut stmt = conn.prepare(
            "SELECT path, name, file_type, size, modified, parent
             FROM files
             WHERE parent = ?1 AND (LOWER(name) LIKE LOWER(?2) OR LOWER(path) LIKE LOWER(?2))
             ORDER BY file_type DESC, name
             LIMIT 1000"
        )?;

        let files = stmt.query_map(params![normalize_path(dir_path), search_pattern], |row| {
            Ok(FileEntry {
                path: PathBuf::from(row.get::<_, String>(0)?),
                name: row.get(1)?,
                file_type: match row.get::<_, String>(2)?.as_str() {
                    "file" => crate::tag_db::FileType::File,
                    "directory" => crate::tag_db::FileType::Directory,
                    _ => crate::tag_db::FileType::File,
                },
                size: row.get(3)?,
                modified: row.get(4)?,
                parent: row.get::<_, Option<String>>(5)?.map(PathBuf::from),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(files)
    }

    pub fn search_by_tag(&self, tag_name: &str, query: &str) -> Result<Vec<FileEntry>> {
        let mut files = self.tag_db.get_files_by_tag(tag_name)?;

        if !query.is_empty() {
            let query_lower = query.to_lowercase();
            files.retain(|f| {
                f.name.to_lowercase().contains(&query_lower) ||
                f.path.to_string_lossy().to_lowercase().contains(&query_lower)
            });
        }

        Ok(files)
    }
}
