mod app;
mod file_associations;
mod indexer;
mod search;
mod tag_db;
mod ui;

use app::FileManagerApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 600.0])
            .with_title("FMS - Find My Shiet"),
        ..Default::default()
    };

    eframe::run_native(
        "FMS",
        options,
        Box::new(|_cc| Box::new(FileManagerApp::new())),
    )
}
