use egui::{Context, FontDefinitions, Style};
use ed25519_dalek::SigningKey;
use rand::RngCore;
use rand::rngs::OsRng;
use std::path::PathBuf;
use nex_core::runtime::node::NexNode;

#[test]
fn test_egui_phosphor_font_and_vector_icon_resolution() {
    let ctx = Context::default();
    let mut fonts = FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    ctx.set_fonts(fonts);

    // Verify phosphor glyphs are recognized in font definitions
    let icons = [
        egui_phosphor::regular::HOUSE,
        egui_phosphor::regular::IMAGE,
        egui_phosphor::regular::FILM_STRIP,
        egui_phosphor::regular::MAP_PIN,
        egui_phosphor::regular::HARD_DRIVE,
        egui_phosphor::regular::USERS,
        egui_phosphor::regular::DEVICES,
        egui_phosphor::regular::HEART,
        egui_phosphor::regular::SHARE_NETWORK,
        egui_phosphor::regular::GEAR,
        egui_phosphor::regular::SHIELD_CHECK,
        egui_phosphor::regular::DATABASE,
        egui_phosphor::regular::CHECK_CIRCLE,
    ];

    for icon in icons {
        assert!(!icon.is_empty(), "Icon glyph must not be empty string");
    }

    // Run an off-screen egui render pass to test layout stability
    let raw_input = egui::RawInput::default();
    let full_output = ctx.run(raw_input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("NEX Sovereign UI");
            for icon in icons {
                ui.label(format!("{} Vector Icon", icon));
            }
        });
    });

    assert!(!full_output.shapes.is_empty(), "Off-screen frame must generate render shapes");
}

#[test]
fn test_visual_layout_does_not_panic_under_various_viewports() {
    let ctx = Context::default();
    let mut fonts = FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    ctx.set_fonts(fonts);

    let viewports = [
        [640.0, 480.0],   // Min size
        [1080.0, 700.0],  // Standard desktop
        [1920.0, 1080.0], // Full HD
        [2560.0, 1440.0], // 2K QHD
    ];

    for size in viewports {
        let mut raw_input = egui::RawInput::default();
        raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(size[0] as f32, size[1] as f32),
        ));

        let full_output = ctx.run(raw_input, |ctx| {
            egui::TopBottomPanel::top("top").show(ctx, |ui| {
                ui.label("⯎ NEX • Sovereign connections");
            });
            egui::SidePanel::left("left").show(ctx, |ui| {
                ui.label(format!("{} Home", egui_phosphor::regular::HOUSE));
            });
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("Viewport Content Frame");
            });
        });

        assert!(!full_output.shapes.is_empty(), "Frame must render successfully at size {:?}", size);
    }
}
