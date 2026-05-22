use eframe::egui;

use crate::app::App;
use crate::config::Theme;
use crate::ui::settings::components::{self as c};
use crate::ui::theme;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let theme = app.cfg.theme;
    page_frame(ui, theme, |ui| {
        c::page_header(ui, theme, "General", Some("Appearance and startup behaviour."));

        c::section(ui, theme, "Appearance", |ui| {
            c::field_row(ui, theme, "Theme", |ui| {
                let prev = app.cfg.theme;
                egui::ComboBox::from_id_salt("settings_theme")
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
                    });
                if app.cfg.theme != prev {
                    theme::apply(ui.ctx(), app.cfg.theme);
                    app.settings_dirty = true;
                }
            });
        });

        c::section(ui, theme, "Startup", |ui| {
            c::field_row(ui, theme, "Launch with Windows", |ui| {
                ui.checkbox(&mut app.cfg.daemon.autostart, "");
            });
            c::field_row(ui, theme, "Start minimized to tray", |ui| {
                ui.checkbox(&mut app.cfg.daemon.start_minimized, "");
            });
        });
    });
}

fn page_frame(ui: &mut egui::Ui, theme: Theme, body: impl FnOnce(&mut egui::Ui)) {
    let t = theme::tokens();
    let p = theme::palette(theme);
    egui::Frame::new()
        .fill(p.paper)
        .inner_margin(egui::Margin {
            left: t.space_xl as i8,
            right: t.space_xl as i8,
            top: 0,
            bottom: t.space_lg as i8,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            body(ui);
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
