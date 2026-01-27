use eframe::egui;
use std::path::PathBuf;
use std::collections::HashMap;

use crate::file_associations::FileAssociations;
use crate::tag_db::FileEntry;
use crate::ui::theme;

const ROW_HEIGHT: f32 = 65.0;
const BUFFER_ITEMS: usize = 5;

pub fn render_file_list(
    ui: &mut egui::Ui,
    files: Vec<FileEntry>,
    mut on_dir_click: Option<&mut dyn FnMut(PathBuf)>,
    selected_index: Option<usize>,
    file_associations: &FileAssociations,
) {
    let available_size = ui.available_size();
    
    if files.is_empty() {
        ui.allocate_ui(available_size, |ui| {
            ui.centered_and_justified(|ui| {
                ui.label("No files found");
            });
        });
        return;
    }

    ui.allocate_ui(available_size, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let viewport_height = ui.available_height();
                let clip_rect = ui.clip_rect();
                let min_rect = ui.min_rect();
                
                let scroll_offset = (clip_rect.min.y - min_rect.min.y).max(0.0);
                let start_index = (scroll_offset / ROW_HEIGHT).floor() as usize;
                let end_index = ((scroll_offset + viewport_height) / ROW_HEIGHT).ceil() as usize;
                
                let visible_start = start_index.saturating_sub(BUFFER_ITEMS);
                let visible_end = (end_index + BUFFER_ITEMS).min(files.len());
                
                if visible_start > 0 {
                    ui.allocate_space(egui::vec2(ui.available_width(), visible_start as f32 * ROW_HEIGHT));
                }
                
                let mut path_string_cache = HashMap::new();
                let mut size_string_cache = HashMap::new();
                
                for index in visible_start..visible_end {
                    let file = &files[index];
                    let is_selected = selected_index == Some(index);
                    let is_dir = matches!(file.file_type, crate::tag_db::FileType::Directory);

                    ui.add_space(4.0);
                    let available_width = ui.available_width();

                    let (row_rect, response) = ui.allocate_exact_size(
                        egui::vec2(available_width, ROW_HEIGHT),
                        egui::Sense::click(),
                    );

                    if response.hovered() {
                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                    }

                    if is_selected {
                        ui.painter().rect_filled(
                            row_rect,
                            0.0,
                            theme::ROW_SELECTED_BG,
                        );
                    } else if response.hovered() {
                        ui.painter().rect_filled(
                            row_rect,
                            0.0,
                            theme::row_hover_bg(),
                        );
                    }

                    let mut content_ui = ui.child_ui(
                        row_rect,
                        egui::Layout::left_to_right(egui::Align::Center),
                    );

                    content_ui.add_space(12.0);

                    let icon_text = if is_dir { "📁" } else { "📄" };
                    let icon_color = if is_dir {
                        theme::ICON_DIRECTORY
                    } else {
                        theme::ICON_FILE
                    };

                    content_ui.label(
                        egui::RichText::new(icon_text)
                            .color(icon_color)
                            .size(20.0),
                    );

                    content_ui.add_space(12.0);

                    content_ui.vertical(|ui| {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(&file.name)
                                .size(14.0)
                                .color(theme::TEXT_PRIMARY),
                        );
                        ui.add_space(2.0);

                        let path_str = path_string_cache.entry(index).or_insert_with(|| {
                            file.path.to_string_lossy().to_string()
                        });
                        ui.label(
                            egui::RichText::new(path_str.as_str())
                                .size(11.0)
                                .color(if is_selected {
                                    theme::TEXT_SECONDARY_SELECTED
                                } else {
                                    theme::TEXT_SECONDARY
                                }),
                        );
                        ui.add_space(4.0);
                    });

                    content_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(12.0);
                        if is_dir {
                            ui.label("—");
                        } else {
                            let size_str = size_string_cache.entry(file.size).or_insert_with(|| {
                                format_size(file.size)
                            });
                            ui.label(size_str.as_str());
                        }
                    });

                    ui.add_space(4.0);

                    if index < files.len() - 1 {
                        ui.separator();
                    }

                    if is_selected {
                        ui.scroll_to_rect(row_rect, Some(egui::Align::Center));
                    }

                    if response.clicked() {
                        if is_dir {
                            if let Some(ref mut on_click) = on_dir_click {
                                on_click(file.path.clone());
                            }
                        } else {
                            let _ = file_associations.open_file(&file.path);
                        }
                    }
                }
                
                let remaining_items = files.len().saturating_sub(visible_end);
                if remaining_items > 0 {
                    ui.allocate_space(egui::vec2(ui.available_width(), remaining_items as f32 * ROW_HEIGHT));
                }
            });
    });
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}
