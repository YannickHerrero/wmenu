use eframe::egui;

use crate::app::App;
use crate::ui::settings::components as c;
use crate::ui::theme;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let theme = app.cfg.theme;
    c::page_frame(ui, theme, |ui| {
        c::page_header(
            ui,
            theme,
            "About",
            Some("wmenu — keyboard-driven launcher, omakase menu, and bindings."),
        );

        let p = theme::palette(theme);
        let t = theme::tokens();

        c::section(ui, theme, "Build", |ui| {
            c::field_row(ui, theme, "Version", |ui| {
                ui.label(
                    egui::RichText::new(format!("wmenu v{}", env!("CARGO_PKG_VERSION")))
                        .color(p.ink)
                        .size(t.font_body),
                );
            });
        });

        c::section(ui, theme, "Files", |ui| {
            let cfg_path = crate::config::config_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "(unknown)".into());
            let folder = crate::config::project_dir().ok();
            c::field_row(ui, theme, "Config file", |ui| {
                if let Some(dir) = folder
                    && ui.button("Open folder").clicked()
                    && let Err(e) = crate::launch::launch(&dir)
                {
                    tracing::warn!("open config folder: {e}");
                }
                ui.label(
                    egui::RichText::new(cfg_path)
                        .monospace()
                        .color(p.ink_soft)
                        .size(t.font_body),
                );
            });
        });
    });
}
