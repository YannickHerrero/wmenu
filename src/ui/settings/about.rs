use eframe::egui;

use crate::app::App;
use crate::ui::settings::components as c;
use crate::ui::settings::{Page, SearchEntry};
use crate::ui::theme;

/// Focus id of the first widget the cross-zone "→ / l" jump should land on.
pub const FIRST_FOCUS: Option<&str> = Some("about_open_folder");

pub const ENTRIES: &[SearchEntry] = &[
    SearchEntry {
        page: Page::About,
        section: "Build",
        label: "Version",
        keywords: &["version", "about", "build", "wmenu"],
        focus_id: None,
    },
    SearchEntry {
        page: Page::About,
        section: "Files",
        label: "Config file",
        keywords: &["config", "file", "path", "toml", "settings", "folder", "open"],
        focus_id: None,
    },
];

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
                if let Some(dir) = folder {
                    let resp = ui.button("Open folder");
                    c::consume_focus_target(
                        &resp,
                        &mut app.focus_target,
                        "about_open_folder",
                    );
                    if resp.clicked()
                        && let Err(e) = crate::launch::launch(&dir)
                    {
                        tracing::warn!("open config folder: {e}");
                    }
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
