use std::path::Path;
use std::sync::Arc;
use walkdir::WalkDir;
use xattr;
use std::time::SystemTime;

use crate::tag_db::{TagDatabase, FileEntry, FileType};

pub struct FileIndexer {
    tag_db: Arc<TagDatabase>,
}

impl FileIndexer {
    pub fn new(tag_db: Arc<TagDatabase>) -> Self {
        FileIndexer { tag_db }
    }

    pub fn index_directory_shallow(&self, dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let entries = std::fs::read_dir(dir)?;
        
        for entry in entries {
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    if let Err(e) = self.index_file(&path) {
                        eprintln!("Error indexing {}: {}", path.display(), e);
                    }
                }
                Err(e) => {
                    eprintln!("Error reading directory entry: {}", e);
                }
            }
        }
        
        Ok(())
    }

    pub fn index_directory_with_depth(&self, root: &Path, max_depth: usize) -> Result<(), Box<dyn std::error::Error>> {
        let walker = WalkDir::new(root)
            .follow_links(false)
            .max_depth(max_depth)
            .into_iter();

        for entry in walker {
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    if let Err(e) = self.index_file(path) {
                        eprintln!("Error indexing {}: {}", path.display(), e);
                    }
                }
                Err(e) => {
                    eprintln!("Error walking directory: {}", e);
                }
            }
        }

        Ok(())
    }

    pub fn index_file(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let metadata = std::fs::metadata(path)?;
        let file_type = if metadata.is_dir() {
            FileType::Directory
        } else {
            FileType::File
        };

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let size = metadata.len();
        let modified = match metadata.modified() {
            Ok(time) => match time.duration_since(SystemTime::UNIX_EPOCH) {
                Ok(duration) => duration.as_secs() as i64,
                Err(e) => {
                    eprintln!("SystemTime before UNIX_EPOCH for {}: {}", path.display(), e);
                    0
                }
            },
            Err(e) => {
                eprintln!("Failed to read modification time for {}: {}", path.display(), e);
                0
            }
        };

        let parent = path.parent().map(|p| p.to_path_buf());

        let file_entry = FileEntry {
            path: path.to_path_buf(),
            name,
            file_type,
            size,
            modified,
            parent,
        };

        self.tag_db.insert_file(&file_entry)?;

        if let Ok(tags) = self.get_macos_tags(path) {
            let path_buf = path.to_path_buf();
            for tag in tags {
                self.tag_db.add_tag_to_file(&path_buf, &tag)?;
            }
        }

        Ok(())
    }

    fn get_macos_tags(&self, path: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let xattr_key = "com.apple.metadata:_kMDItemUserTags";
        
        if let Ok(Some(value)) = xattr::get(path, xattr_key) {
            use std::io::Cursor;
            let mut cursor = Cursor::new(&value);
            if let Ok(plist_value) = plist::Value::from_reader(&mut cursor) {
                if let plist::Value::Array(tags) = plist_value {
                    return Ok(tags
                        .iter()
                        .filter_map(|v| {
                            if let plist::Value::String(s) = v {
                                Some(s.clone())
                            } else {
                                None
                            }
                        })
                        .collect());
                }
            }
        }

        Ok(vec![])
    }
}
