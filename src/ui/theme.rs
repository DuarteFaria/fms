use eframe::egui;

pub const ROW_SELECTED_BG: egui::Color32 = egui::Color32::from_rgb(50, 50, 50);
pub fn row_hover_bg() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 2)
}

pub const ICON_DIRECTORY: egui::Color32 = egui::Color32::from_rgb(0, 122, 255);
pub const ICON_FILE: egui::Color32 = egui::Color32::from_rgb(153, 153, 153);

pub const TEXT_PRIMARY: egui::Color32 = egui::Color32::WHITE;
pub const TEXT_SECONDARY_SELECTED: egui::Color32 = egui::Color32::from_rgb(200, 200, 200);
pub const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(102, 102, 102);

pub const TREE_CURRENT_STROKE: egui::Color32 = egui::Color32::from_rgb(100, 150, 255);
