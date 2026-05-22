use eframe::egui;

use crate::app::App;
use crate::config::Theme;
use crate::ui::settings::components as c;
use crate::ui::settings::{Page, SearchEntry};
use crate::ui::theme;

#[allow(dead_code)] // consumed by the search-results view in the next commit
pub const ENTRIES: &[SearchEntry] = &[
    SearchEntry {
        page: Page::General,
        section: "Appearance",
        label: "Theme",
        keywords: &["theme", "appearance", "color", "colour", "dark", "light", "paper", "ink"],
        focus_id: Some("general_theme"),
    },
    SearchEntry {
        page: Page::General,
        section: "Startup",
        label: "Launch with Windows",
        keywords: &["autostart", "startup", "boot", "windows", "launch"],
        focus_id: Some("general_autostart"),
    },
    SearchEntry {
        page: Page::General,
        section: "Startup",
        label: "Start minimized to tray",
        keywords: &["minimized", "tray", "startup", "background", "hidden"],
        focus_id: Some("general_start_minimized"),
    },
];

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let theme = app.cfg.theme;
    c::page_frame(ui, theme, |ui| {
        c::page_header(ui, theme, "General", Some("Appearance and startup behaviour."));

        c::section(ui, theme, "Appearance", |ui| {
            c::field_row(ui, theme, "Theme", |ui| {
                let prev = app.cfg.theme;
                let resp = egui::ComboBox::from_id_salt(c::focus_id("general_theme"))
                    .selected_text(theme_label(app.cfg.theme))
                    .show_ui(ui, |ui| {
                        for t in [
                            Theme::Paper,
                            Theme::Stone,
                            Theme::Sage,
                            Theme::Clay,
                            Theme::Ink,
                        ] {
                            ui.selectable_value(&mut app.cfg.theme, t, theme_label(t));
                        }
                    })
                    .response;
                c::consume_focus_target(&resp, &mut app.focus_target, "general_theme");
                if app.cfg.theme != prev {
                    theme::apply(ui.ctx(), app.cfg.theme);
                    app.settings_dirty = true;
                }
            });
        });

        c::section(ui, theme, "Startup", |ui| {
            c::field_row(ui, theme, "Launch with Windows", |ui| {
                let resp = ui.checkbox(&mut app.cfg.daemon.autostart, "");
                c::consume_focus_target(&resp, &mut app.focus_target, "general_autostart");
            });
            c::field_row(ui, theme, "Start minimized to tray", |ui| {
                let resp = ui.checkbox(&mut app.cfg.daemon.start_minimized, "");
                c::consume_focus_target(
                    &resp,
                    &mut app.focus_target,
                    "general_start_minimized",
                );
            });
        });
    });
}

fn theme_label(t: Theme) -> &'static str {
    match t {
        Theme::Paper => "Paper",
        Theme::Stone => "Stone",
        Theme::Sage => "Sage",
        Theme::Clay => "Clay",
        Theme::Ink => "Ink",
    }
}
