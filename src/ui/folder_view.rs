use eframe::egui;
use std::path::PathBuf;

use crate::file_associations::FileAssociations;
use crate::tag_db::FileEntry;
use crate::ui::file_list::render_file_list;

pub fn render_folder_view(
    files: Vec<FileEntry>,
    current_path: PathBuf,
    on_path_change: &mut dyn FnMut(PathBuf),
    selected_file_index: Option<usize>,
    file_associations: &FileAssociations,
    ui: &mut egui::Ui,
) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            let components: Vec<_> = current_path.components().collect();
            for (i, component) in components.iter().enumerate() {
                if i > 0 {
                    ui.label(" / ");
                }
                let path = components[..=i]
                    .iter()
                    .collect::<PathBuf>();
                let name = component.as_os_str().to_string_lossy().to_string();

                if ui.link(&name).clicked() {
                    on_path_change(path.clone());
                }
            }
        });
        ui.separator();

        ui.allocate_ui(ui.available_size(), |ui| {
            render_file_list(ui, files, Some(on_path_change), selected_file_index, file_associations);
        });
    });
}
