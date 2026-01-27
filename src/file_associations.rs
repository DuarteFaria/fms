use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    #[serde(flatten)]
    associations: HashMap<String, String>,
}

pub struct FileAssociations {
    associations: HashMap<String, String>,
    config_path: PathBuf,
}

impl FileAssociations {
    pub fn new() -> Self {
        let home_dir = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/"));
        
        let config_dir = home_dir.join(".fms");
        let config_path = config_dir.join("apps.json");
        
        let associations = Self::load_config(&config_path);
        
        FileAssociations {
            associations,
            config_path,
        }
    }
    
    fn load_config(config_path: &Path) -> HashMap<String, String> {
        if !config_path.exists() {
            if let Some(parent) = config_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            return HashMap::new();
        }
        
        match std::fs::read_to_string(config_path) {
            Ok(content) => {
                match serde_json::from_str::<Config>(&content) {
                    Ok(config) => config.associations,
                    Err(e) => {
                        eprintln!("Error parsing config file {}: {}", config_path.display(), e);
                        HashMap::new()
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading config file {}: {}", config_path.display(), e);
                HashMap::new()
            }
        }
    }
    
    pub fn open_file(&self, file_path: &Path) -> std::io::Result<std::process::Output> {
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase());
        
        if let Some(ext) = extension {
            if let Some(app_name) = self.associations.get(&ext) {
                return Command::new("open")
                    .arg("-a")
                    .arg(app_name)
                    .arg(file_path)
                    .output();
            }
        }
        
        Command::new("open")
            .arg(file_path)
            .output()
    }
}
