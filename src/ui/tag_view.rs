use eframe::egui;
use std::sync::Arc;

use crate::file_associations::FileAssociations;
use crate::tag_db::{TagDatabase, FileEntry};
use crate::ui::file_list::render_file_list;
use crate::ui::theme;

pub fn render_tag_view(
    tag_db: Arc<TagDatabase>,
    files: Vec<FileEntry>,
    selected_tag: Option<String>,
    on_tag_select: &mut dyn FnMut(Option<String>),
    selected_file_index: Option<usize>,
    file_associations: &FileAssociations,
    ui: &mut egui::Ui,
) {
    let tags_result = tag_db.get_all_tags();
    let tags = tags_result.unwrap_or_default();

    ui.horizontal(|ui| {
        egui::SidePanel::left("tag_list")
            .resizable(true)
            .default_width(200.0)
            .show_inside(ui, |ui| {
                ui.vertical(|ui| {
                    if ui.selectable_label(selected_tag.is_none(), "All Files").clicked() {
                        on_tag_select(None);
                    }
                    ui.separator();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for tag in tags {
                            let tag_name = tag.name.clone();
                            let file_count = tag.file_count;
                            let is_selected = selected_tag.as_ref() == Some(&tag_name);

                            ui.horizontal(|ui| {
                                if ui.selectable_label(is_selected, &tag_name).clicked() {
                                    on_tag_select(Some(tag_name.clone()));
                                }
                                ui.label(
                                    egui::RichText::new(file_count.to_string())
                                        .size(10.0)
                                        .color(if is_selected {
                                            theme::TEXT_PRIMARY
                                        } else {
                                            theme::TEXT_SECONDARY
                                        })
                                );
                            });
                        }
                    });
                });
            });

        ui.vertical(|ui| {
            ui.allocate_ui(ui.available_size(), |ui| {
                render_file_list(ui, files, None, selected_file_index, file_associations);
            });
        });
    });
}
