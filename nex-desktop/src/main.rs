#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod ui;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("NEX")
            .with_inner_size([1080.0, 700.0])
            .with_min_inner_size([640.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "NEX",
        native_options,
        Box::new(|cc| Ok(Box::new(app::NexDesktopApp::new(cc)))),
    )
}

