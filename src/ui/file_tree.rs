use eframe::egui;
use std::collections::HashSet;
use std::path::PathBuf;

use crate::tag_db::{TagDatabase, FileEntry, FileType};
use crate::ui::theme;

pub fn render_file_tree(
    ui: &mut egui::Ui,
    tag_db: &TagDatabase,
    root_path: &PathBuf,
    current_path: &PathBuf,
    expanded: &mut HashSet<PathBuf>,
    show_hidden_files: bool,
    on_path_click: &mut dyn FnMut(PathBuf),
    max_width: f32,
) {
    let root_entry = match tag_db.get_directory(root_path) {
        Ok(Some(entry)) => entry,
        Ok(None) => {
            if *root_path == PathBuf::from("/") {
                FileEntry {
                    path: PathBuf::from("/"),
                    name: "/".to_string(),
                    file_type: crate::tag_db::FileType::Directory,
                    size: 0,
                    modified: 0,
                    parent: None,
                }
            } else {
                ui.label("No root directory found");
                return;
            }
        }
        Err(e) => {
            ui.label(format!("Error: {}", e));
            return;
        }
    };

    let child_dirs = get_child_directories(tag_db, root_path, show_hidden_files);
    let is_last = true;
    
    render_directory(
        ui,
        tag_db,
        &root_entry,
        current_path,
        expanded,
        show_hidden_files,
        on_path_click,
        0,
        max_width,
        is_last,
        Vec::new(),
        root_path,
    );
}

fn render_directory(
    ui: &mut egui::Ui,
    tag_db: &TagDatabase,
    dir: &FileEntry,
    current_path: &PathBuf,
    expanded: &mut HashSet<PathBuf>,
    show_hidden_files: bool,
    on_path_click: &mut dyn FnMut(PathBuf),
    depth: usize,
    max_width: f32,
    is_last: bool,
    parent_prefix: Vec<bool>,
    tree_root: &PathBuf,
) {
    if !show_hidden_files && dir.name.starts_with('.') {
        return;
    }
    
    let is_expanded = expanded.contains(&dir.path);
    let is_current = dir.path == *current_path;
    let has_children = has_child_directories(tag_db, &dir.path, show_hidden_files);
    let child_dirs = if has_children {
        get_child_directories(tag_db, &dir.path, show_hidden_files)
    } else {
        vec![]
    };

    let row_height = ui.text_style_height(&egui::TextStyle::Body) + 4.0;
    
    let mut label_response_opt = None;
    
    ui.allocate_ui(
        egui::vec2(max_width, row_height),
        |ui| {
            let mut prefix_string = String::new();
            for &is_parent_last in &parent_prefix {
                if is_parent_last {
                    prefix_string.push_str("   ");
                } else {
                    prefix_string.push_str("│  ");
                }
            }
            
            if depth > 0 {
                if is_last {
                    prefix_string.push_str("└─");
                } else {
                    prefix_string.push_str("├─");
                }
            }
            
            ui.horizontal(|ui| {
                if !prefix_string.is_empty() {
                    ui.label(
                        egui::RichText::new(prefix_string)
                            .monospace()
                    );
                }

                if has_children {
                    let expand_char = if is_expanded { "−" } else { "+" };
                    let expand_response = ui.label(
                        egui::RichText::new(expand_char)
                            .monospace()
                            .size(14.0)
                    );
                    if expand_response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if expand_response.clicked() {
                        if is_expanded {
                            expanded.remove(&dir.path);
                        } else {
                            expanded.insert(dir.path.clone());
                        }
                    }
                } else {
                    ui.add_space(12.0);
                }

                ui.add_space(2.0);

                let display_name = if dir.path == *tree_root {
                    "/".to_string()
                } else {
                    dir.name.clone()
                };
                
                let label_text = if display_name.len() > 20 {
                    format!("{}...", &display_name[..17])
                } else {
                    display_name
                };
                
                let label_response = ui.label(&label_text);
                if label_response.hovered() {
                    let rect = label_response.rect.expand(2.0);
                    ui.painter().rect_filled(
                        rect,
                        0.0,
                        theme::row_hover_bg(),
                    );
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                label_response_opt = Some(label_response);
            });
        }
    );

    if let Some(label_response) = label_response_opt {
        if label_response.clicked() {
            on_path_click(dir.path.clone());
        }

        if is_current {
            let rect = label_response.rect.expand(4.0);
            ui.painter().rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(2.0, theme::TREE_CURRENT_STROKE),
            );
        }
    }

    if is_expanded && has_children {
        let mut new_prefix = parent_prefix.clone();
        if depth > 0 {
            new_prefix.push(!is_last);
        }
        
        for (idx, child_dir) in child_dirs.iter().enumerate() {
            let is_child_last = idx == child_dirs.len() - 1;
            render_directory(
                ui,
                tag_db,
                child_dir,
                current_path,
                expanded,
                show_hidden_files,
                on_path_click,
                depth + 1,
                max_width,
                is_child_last,
                new_prefix.clone(),
                tree_root,
            );
        }
    }
}

fn has_child_directories(tag_db: &TagDatabase, dir_path: &PathBuf, show_hidden_files: bool) -> bool {
    if let Ok(files) = tag_db.get_files_in_directory(dir_path) {
        files.iter().any(|f| {
            matches!(f.file_type, FileType::Directory) && (show_hidden_files || !f.name.starts_with('.'))
        })
    } else {
        false
    }
}

fn get_child_directories(tag_db: &TagDatabase, dir_path: &PathBuf, show_hidden_files: bool) -> Vec<FileEntry> {
    if let Ok(files) = tag_db.get_files_in_directory(dir_path) {
        let mut dirs: Vec<FileEntry> = files
            .into_iter()
            .filter(|f| matches!(f.file_type, FileType::Directory))
            .filter(|f| show_hidden_files || !f.name.starts_with('.'))
            .collect();
        dirs.sort_by(|a, b| a.name.cmp(&b.name));
        dirs
    } else {
        vec![]
    }
}
