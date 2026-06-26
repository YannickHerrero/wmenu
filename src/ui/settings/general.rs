use eframe::egui;

use crate::app::App;
use crate::config::Theme;
use crate::theme_orchestrator;
use crate::ui::settings::components as c;
use crate::ui::settings::{Page, SearchEntry};
use crate::ui::theme;

/// Focus id of the first widget the cross-zone "→ / l" jump should land on.
pub const FIRST_FOCUS: Option<&str> = Some("general_theme");

pub const ENTRIES: &[SearchEntry] = &[
    SearchEntry {
        page: Page::General,
        section: "Appearance",
        label: "Theme",
        keywords: &[
            "theme",
            "appearance",
            "color",
            "colour",
            "dark",
            "light",
            "paper",
            "ink",
        ],
        focus_id: Some("general_theme"),
    },
    SearchEntry {
        page: Page::General,
        section: "Appearance",
        label: "Monochromatic terminal colors",
        keywords: &[
            "terminal",
            "monochrome",
            "monochromatic",
            "ansi",
            "wezterm",
            "windows terminal",
            "colors",
            "colours",
        ],
        focus_id: Some("general_terminal_monochrome"),
    },
    SearchEntry {
        page: Page::General,
        section: "Appearance",
        label: "Borderless settings window",
        keywords: &[
            "borderless",
            "titlebar",
            "chrome",
            "decorations",
            "frameless",
            "settings",
            "window",
        ],
        focus_id: Some("general_borderless"),
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
        c::page_header(
            ui,
            theme,
            "General",
            Some("Appearance and startup behaviour."),
        );

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
                    let palette = theme::palette(app.cfg.theme);
                    if let Err(e) = theme_orchestrator::apply_terminal_colors(
                        app.cfg.theme,
                        &palette,
                        app.cfg.terminal_monochrome,
                    ) {
                        tracing::warn!("apply terminal colors after theme change: {e}");
                    }
                }
            });
            c::field_row(ui, theme, "Monochromatic terminal colors", |ui| {
                let resp = ui.checkbox(&mut app.cfg.terminal_monochrome, "");
                c::consume_focus_target(
                    &resp,
                    &mut app.focus_target,
                    "general_terminal_monochrome",
                );
                if resp.changed() {
                    let palette = theme::palette(app.cfg.theme);
                    if let Err(e) = theme_orchestrator::apply_terminal_colors(
                        app.cfg.theme,
                        &palette,
                        app.cfg.terminal_monochrome,
                    ) {
                        tracing::warn!("apply terminal colors: {e}");
                    }
                }
            });
            c::field_row(ui, theme, "Borderless settings window", |ui| {
                let resp = ui.checkbox(&mut app.cfg.settings_borderless, "");
                c::consume_focus_target(&resp, &mut app.focus_target, "general_borderless");
            });
        });

        c::section(ui, theme, "Startup", |ui| {
            c::field_row(ui, theme, "Launch with Windows", |ui| {
                let resp = ui.checkbox(&mut app.cfg.daemon.autostart, "");
                c::consume_focus_target(&resp, &mut app.focus_target, "general_autostart");
            });
            c::field_row(ui, theme, "Start minimized to tray", |ui| {
                let resp = ui.checkbox(&mut app.cfg.daemon.start_minimized, "");
                c::consume_focus_target(&resp, &mut app.focus_target, "general_start_minimized");
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
