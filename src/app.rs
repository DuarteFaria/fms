use eframe::egui;
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use sysinfo::{System, Pid};

use crate::file_associations::FileAssociations;
use crate::indexer::FileIndexer;
use crate::search::SearchEngine;
use crate::tag_db::TagDatabase;

pub struct FileManagerApp {
    indexer: Arc<FileIndexer>,
    search_engine: Arc<SearchEngine>,
    tag_db: Arc<TagDatabase>,
    file_associations: FileAssociations,
    current_view: ViewTab,
    search_query: String,
    is_indexing: Arc<AtomicBool>,
    folder_current_path: PathBuf,
    tag_selected: Option<String>,
    indexing_thread: Option<std::thread::JoinHandle<()>>,
    last_indexed_path: PathBuf,
    search_field_id: egui::Id,
    system: System,
    last_update: Instant,
    process_id: Pid,
    selected_file_index: Option<usize>,
    last_search_query: String,
    directory_search_mode: bool,
    show_hidden_files: bool,
    expanded_directories: HashSet<PathBuf>,
    tree_root_path: PathBuf,
    show_directory_tree: bool,
    creating_entry: Option<CreatingEntryKind>,
    new_entry_name: String,
}

impl Drop for FileManagerApp {
    fn drop(&mut self) {
        if let Some(handle) = self.indexing_thread.take() {
            let _ = handle.join();
        }
    }
}

fn handle_list_navigation(
    input: &egui::InputState,
    selected_index: &mut Option<usize>,
    len: usize,
) {
    if input.key_pressed(egui::Key::ArrowDown) {
        *selected_index = Some(
            selected_index
                .map(|i| (i + 1).min(len.saturating_sub(1)))
                .unwrap_or(0),
        );
    }

    if input.key_pressed(egui::Key::ArrowUp) {
        *selected_index = selected_index
            .map(|i| i.saturating_sub(1))
            .or_else(|| if len > 0 { Some(len - 1) } else { None });
    }
}

#[derive(Clone, Copy, PartialEq)]
enum ViewTab {
    Folders,
    Tags,
}

#[derive(Clone, Copy)]
enum CreatingEntryKind {
    NewFile,
    NewDirectory,
}

impl eframe::App for FileManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let input = ctx.input(|i| i.clone());
        
        if self.search_query != self.last_search_query {
            self.selected_file_index = None;
            self.last_search_query = self.search_query.clone();
        }
        
        if input.key_pressed(egui::Key::F) && (input.modifiers.command || input.modifiers.ctrl) {
            ctx.memory_mut(|m| m.request_focus(self.search_field_id));
        }

        if self.current_view == ViewTab::Folders {
            if input.key_pressed(egui::Key::N) && (input.modifiers.command || input.modifiers.ctrl) {
                self.creating_entry = Some(CreatingEntryKind::NewFile);
                self.new_entry_name.clear();
            }

            if input.key_pressed(egui::Key::D) && (input.modifiers.command || input.modifiers.ctrl) {
                self.creating_entry = Some(CreatingEntryKind::NewDirectory);
                self.new_entry_name.clear();
            }

            if input.key_pressed(egui::Key::F)
                && (input.modifiers.command || input.modifiers.ctrl)
                && input.modifiers.alt
            {
                self.directory_search_mode = !self.directory_search_mode;
                self.selected_file_index = None;
            }
        }

        if input.key_pressed(egui::Key::Num1) && (input.modifiers.command || input.modifiers.ctrl) {
            self.current_view = ViewTab::Folders;
            self.selected_file_index = None;
        }

        if input.key_pressed(egui::Key::Num2) && (input.modifiers.command || input.modifiers.ctrl) {
            self.current_view = ViewTab::Tags;
            self.selected_file_index = None;
        }

        if input.key_pressed(egui::Key::Escape) {
            if ctx.memory(|m| m.has_focus(self.search_field_id)) {
                self.search_query.clear();
                ctx.memory_mut(|m| m.surrender_focus(self.search_field_id));
            }
        }

        if input.key_pressed(egui::Key::B) && (input.modifiers.command || input.modifiers.ctrl) {
            if self.current_view == ViewTab::Folders {
                self.show_directory_tree = !self.show_directory_tree;
            }
        }

        let should_collapse_folders = (input.key_pressed(egui::Key::K) && (input.modifiers.command || input.modifiers.ctrl))
            || (input.key_pressed(egui::Key::Period) && (input.modifiers.command || input.modifiers.ctrl) && input.modifiers.shift);
        
        if should_collapse_folders {
            if self.current_view == ViewTab::Folders {
                self.expanded_directories.clear();
                let mut path_to_expand = self.folder_current_path.clone();
                
                while let Some(parent) = path_to_expand.parent() {
                    let parent_path = parent.to_path_buf();
                    if parent_path.starts_with(&self.tree_root_path) {
                        if parent_path == self.tree_root_path {
                            self.expanded_directories.insert(self.tree_root_path.clone());
                            break;
                        } else {
                            self.expanded_directories.insert(parent_path.clone());
                        }
                    }
                    path_to_expand = parent_path;
                    if path_to_expand == self.tree_root_path {
                        break;
                    }
                }
            }
        }

        if input.key_pressed(egui::Key::Period) && (input.modifiers.command || input.modifiers.ctrl) && !input.modifiers.shift {
            self.show_hidden_files = !self.show_hidden_files;
            self.selected_file_index = None;
        }

        egui::TopBottomPanel::top("top_panel")
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.current_view, ViewTab::Folders, "Folders");
                    ui.selectable_value(&mut self.current_view, ViewTab::Tags, "Tags");
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let hint = if self.current_view == ViewTab::Folders && self.directory_search_mode {
                            "Search in directory..."
                        } else {
                            "Search files..."
                        };
                        ui.add(egui::TextEdit::singleline(&mut self.search_query)
                            .id(self.search_field_id)
                            .hint_text(hint)
                            .desired_width(300.0));
                    });
                });
            });

        if self.current_view == ViewTab::Folders && self.folder_current_path != self.last_indexed_path {
            let path_to_index = self.folder_current_path.clone();
            let indexer = self.indexer.clone();
            std::thread::spawn(move || {
                if let Err(e) = indexer.index_directory_shallow(&path_to_index) {
                    eprintln!("Error indexing directory: {}", e);
                }
            });
            self.last_indexed_path = self.folder_current_path.clone();
            
            let mut path_to_expand = self.folder_current_path.clone();
            while let Some(parent) = path_to_expand.parent() {
                self.expanded_directories.insert(parent.to_path_buf());
                path_to_expand = parent.to_path_buf();
            }
        }

        if self.current_view == ViewTab::Folders && self.show_directory_tree {
            let tree_root = self.tree_root_path.clone();
            let current_path = self.folder_current_path.clone();
            let tag_db = self.tag_db.clone();
            let mut expanded_dirs = std::mem::take(&mut self.expanded_directories);
            let show_hidden = self.show_hidden_files;
            let mut path_to_set: Option<PathBuf> = None;
            
            let mut path_to_expand = current_path.clone();
            while let Some(parent) = path_to_expand.parent() {
                let parent_path = parent.to_path_buf();
                if parent_path.starts_with(&tree_root) {
                    expanded_dirs.insert(parent_path.clone());
                    if parent_path == tree_root {
                        break;
                    }
                }
                path_to_expand = parent_path;
            }
            
            egui::SidePanel::left("folder_tree")
                .resizable(false)
                .default_width(150.0)
                .min_width(150.0)
                .max_width(150.0)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.heading("Directory tree");
                        ui.separator();
                        
                        let height = ui.available_size().y;
                        let available_size = egui::vec2(150.0, height);
                        ui.allocate_ui(available_size, |ui| {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.allocate_ui(
                                        egui::vec2(150.0, ui.available_height()),
                                        |ui| {
                                            crate::ui::file_tree::render_file_tree(
                                                ui,
                                                &tag_db,
                                                &tree_root,
                                                &current_path,
                                                &mut expanded_dirs,
                                                show_hidden,
                                                &mut |path| {
                                                    path_to_set = Some(path.clone());
                                                },
                                                150.0,
                                            );
                                        }
                                    );
                                });
                        });
                    });
                });
            
            if let Some(path) = path_to_set {
                self.folder_current_path = path.clone();
                self.selected_file_index = None;
                
                let mut path_to_expand = path;
                while let Some(parent) = path_to_expand.parent() {
                    expanded_dirs.insert(parent.to_path_buf());
                    path_to_expand = parent.to_path_buf();
                    if path_to_expand == self.tree_root_path {
                        break;
                    }
                }
            }
            self.expanded_directories = expanded_dirs;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_view {
                ViewTab::Folders => {
                    let current_path = self.folder_current_path.clone();
                    let files_result = if self.search_query.is_empty() {
                        self.tag_db.get_files_in_directory(&current_path)
                    } else if self.directory_search_mode {
                        self.search_engine.search_in_directory(&current_path, &self.search_query)
                    } else {
                        self.search_engine.search(&self.search_query)
                    };
                    let mut files = files_result.unwrap_or_default();
                    
                    if !self.show_hidden_files {
                        files.retain(|file| !file.name.starts_with('.'));
                    }
                    
                    let input = ctx.input(|i| i.clone());
                    let files_len = files.len();

                    if files_len > 0 {
                        handle_list_navigation(&input, &mut self.selected_file_index, files_len);
                    } else {
                        self.selected_file_index = None;
                    }
                    
                    if input.key_pressed(egui::Key::Enter) {
                        if let Some(idx) = self.selected_file_index {
                            if let Some(file) = files.get(idx) {
                                let is_dir = matches!(file.file_type, crate::tag_db::FileType::Directory);
                                if is_dir {
                                    self.folder_current_path = file.path.clone();
                                    self.selected_file_index = None;
                                } else {
                                    let _ = self.file_associations.open_file(&file.path);
                                }
                            }
                        }
                    }
                    
                    if (input.key_pressed(egui::Key::ArrowLeft) || input.key_pressed(egui::Key::Backspace)) 
                        && !ctx.memory(|m| m.has_focus(self.search_field_id)) {
                        if let Some(parent) = self.folder_current_path.parent() {
                            self.folder_current_path = parent.to_path_buf();
                            self.selected_file_index = None;
                        }
                    }
                    
                    let selected_index = self.selected_file_index;
                    let current_path = self.folder_current_path.clone();
                    let search_query_empty = self.search_query.is_empty();
                    let tree_root = self.tree_root_path.clone();
                    let tag_db = self.tag_db.clone();
                    let mut expanded_dirs = std::mem::take(&mut self.expanded_directories);
                    let mut path_to_expand_after: Option<PathBuf> = None;
                    crate::ui::folder_view::render_folder_view(
                        files,
                        current_path,
                        &mut |path| {
                            self.folder_current_path = path.clone();
                            self.selected_file_index = None;
                            path_to_expand_after = Some(path);
                        },
                        selected_index,
                        &self.file_associations,
                        ui,
                    );
                    if let Some(path) = path_to_expand_after {
                        let mut path_to_expand = path.clone();
                        while let Some(parent) = path_to_expand.parent() {
                            expanded_dirs.insert(parent.to_path_buf());
                            path_to_expand = parent.to_path_buf();
                        }
                    }
                    self.expanded_directories = expanded_dirs;
                }
                ViewTab::Tags => {
                    let files_result = if let Some(tag) = &self.tag_selected {
                        self.search_engine.search_by_tag(tag, &self.search_query)
                    } else if self.search_query.is_empty() {
                        Ok(vec![])
                    } else {
                        self.search_engine.search(&self.search_query)
                    };
                    let mut files = files_result.unwrap_or_default();
                    
                    if !self.show_hidden_files {
                        files.retain(|file| !file.name.starts_with('.'));
                    }
                    
                    let input = ctx.input(|i| i.clone());
                    let files_len = files.len();

                    if files_len > 0 {
                        handle_list_navigation(&input, &mut self.selected_file_index, files_len);
                    } else {
                        self.selected_file_index = None;
                    }
                    
                    if input.key_pressed(egui::Key::Enter) {
                        if let Some(idx) = self.selected_file_index {
                            if let Some(file) = files.get(idx) {
                                let _ = self.file_associations.open_file(&file.path);
                            }
                        }
                    }
                    
                    let selected_index = self.selected_file_index;
                    crate::ui::tag_view::render_tag_view(
                        self.tag_db.clone(),
                        files,
                        self.tag_selected.clone(),
                        &mut |tag| {
                            self.tag_selected = tag;
                            self.selected_file_index = None;
                        },
                        selected_index,
                        &self.file_associations,
                        ui,
                    );
                }
            }
        });

        if let Some(kind) = self.creating_entry {
            let mut create_now = false;
            let mut cancel = false;

            egui::Window::new(match kind {
                CreatingEntryKind::NewFile => "New file",
                CreatingEntryKind::NewDirectory => "New directory",
            })
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label("Name:");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.new_entry_name)
                            .desired_width(200.0),
                    );
                    if !response.has_focus() {
                        response.request_focus();
                    }

                    ui.horizontal(|ui| {
                        let can_create = !self.new_entry_name.trim().is_empty();
                        if ui
                            .add_enabled(can_create, egui::Button::new("Create"))
                            .clicked()
                        {
                            if can_create {
                                create_now = true;
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
            });

            if input.key_pressed(egui::Key::Escape) {
                cancel = true;
            }

            if input.key_pressed(egui::Key::Enter) && !self.new_entry_name.trim().is_empty() {
                create_now = true;
            }

            if cancel {
                self.creating_entry = None;
                self.new_entry_name.clear();
            } else if create_now {
                let name = self.new_entry_name.trim().to_string();
                if !name.is_empty() {
                    match kind {
                        CreatingEntryKind::NewFile => {
                            self.create_file_in_current(&name);
                        }
                        CreatingEntryKind::NewDirectory => {
                            self.create_directory_in_current(&name);
                        }
                    }
                }
                self.creating_entry = None;
                self.new_entry_name.clear();
            }
        }

        if self.last_update.elapsed() > Duration::from_millis(500) {
            self.system.refresh_process(self.process_id);
            self.system.refresh_memory();
            self.last_update = Instant::now();
        }

        let memory_mb = if let Some(process) = self.system.process(self.process_id) {
            process.memory() as f64 / 1024.0 / 1024.0
        } else {
            0.0
        };
        let cpu_usage = if let Some(process) = self.system.process(self.process_id) {
            process.cpu_usage() as f64
        } else {
            0.0
        };

        egui::TopBottomPanel::bottom("status_bar")
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if self.is_indexing.load(Ordering::Relaxed) {
                        ui.label("Indexing files...");
                    } else {
                        ui.label("Ready");
                    }
                    ui.separator();
                    ui.label(format!("Memory: {:.1} MB", memory_mb));
                    ui.separator();
                    ui.label(format!("CPU: {:.1}%", cpu_usage));
                });
            });
    }
}

impl FileManagerApp {
    pub fn new() -> Self {
        let home_dir = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/"));

        let tag_db = Arc::new(TagDatabase::new().expect("Failed to create tag database"));
        let indexer = Arc::new(FileIndexer::new(tag_db.clone()));
        let search_engine = Arc::new(SearchEngine::new(tag_db.clone()));
        let file_associations = FileAssociations::new();
        let is_indexing = Arc::new(AtomicBool::new(true));

        let is_indexing_clone = is_indexing.clone();
        let indexer_clone = indexer.clone();
        let home_dir_clone = home_dir.clone();
        let root_path = PathBuf::from("/");

        let indexing_thread = Some(std::thread::spawn(move || {
            if let Err(e) = indexer_clone.index_file(&root_path) {
                eprintln!("Error indexing root directory entry: {}", e);
            }
            if let Err(e) = indexer_clone.index_directory_shallow(&root_path) {
                eprintln!("Error indexing root directory: {}", e);
            }
            if let Err(e) = indexer_clone.index_directory_with_depth(&home_dir_clone, 3) {
                eprintln!("Error indexing directory: {}", e);
            }
            is_indexing_clone.store(false, Ordering::Relaxed);
        }));

        let mut system = System::new();
        let process_id = Pid::from_u32(std::process::id());
        system.refresh_process(process_id);
        system.refresh_memory();

        FileManagerApp {
            indexer,
            search_engine,
            tag_db,
            file_associations,
            current_view: ViewTab::Folders,
            search_query: String::new(),
            is_indexing,
            folder_current_path: home_dir.clone(),
            tag_selected: None,
            indexing_thread,
            last_indexed_path: PathBuf::new(),
            search_field_id: egui::Id::new("search_field"),
            system,
            last_update: Instant::now(),
            process_id,
            selected_file_index: None,
            last_search_query: String::new(),
            directory_search_mode: false,
            show_hidden_files: false,
            expanded_directories: HashSet::new(),
            tree_root_path: PathBuf::from("/"),
            show_directory_tree: true,
            creating_entry: None,
            new_entry_name: String::new(),
        }
    }

    fn create_directory_in_current(&mut self, name: &str) {
        let mut path = self.folder_current_path.clone();
        path.push(name);
        if let Err(e) = fs::create_dir(&path) {
            eprintln!("Error creating directory {:?}: {}", path, e);
        } else {
            self.refresh_current_directory();
        }
    }

    fn create_file_in_current(&mut self, name: &str) {
        let mut path = self.folder_current_path.clone();
        path.push(name);
        match File::create(&path) {
            Ok(_) => {
                self.refresh_current_directory();
            }
            Err(e) => {
                eprintln!("Error creating file {:?}: {}", path, e);
            }
        }
    }

    fn refresh_current_directory(&mut self) {
        let path_to_index = self.folder_current_path.clone();
        let indexer = self.indexer.clone();
        std::thread::spawn(move || {
            if let Err(e) = indexer.index_directory_shallow(&path_to_index) {
                eprintln!("Error indexing directory after create: {}", e);
            }
        });
    }
}
