#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod ui;

fn main() -> eframe::Result<()> {
    let icon_data = eframe::icon_data::from_png_bytes(include_bytes!("../assets/app_icon.png")).ok();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("NEX")
        .with_inner_size([1080.0, 700.0])
        .with_min_inner_size([640.0, 480.0]);

    if let Some(icon) = icon_data {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "NEX",
        native_options,
        Box::new(|cc| Ok(Box::new(app::NexDesktopApp::new(cc)))),
    )
}

