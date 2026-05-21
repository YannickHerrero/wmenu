use eframe::egui;

use crate::app::App;

pub fn show(_app: &mut App, ui: &mut egui::Ui) {
    ui.heading("About");
    ui.add_space(8.0);
    ui.label(format!("wmenu v{}", env!("CARGO_PKG_VERSION")));
    ui.add_space(8.0);
    let path = crate::config::config_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "(unknown)".into());
    ui.label(egui::RichText::new(format!("Config: {path}")).small().weak());
}
