use std::path::PathBuf;

use eframe::egui;

use crate::app::App;
use crate::config::Theme;
use crate::ui::settings::components as c;
use crate::ui::settings::{Page, SearchEntry};
use crate::ui::theme;

/// Focus id of the first widget the cross-zone "→ / l" jump should land on.
pub const FIRST_FOCUS: Option<&str> = Some("launcher_hotkey_input");

pub const ENTRIES: &[SearchEntry] = &[
    SearchEntry {
        page: Page::Launcher,
        section: "Hotkeys",
        label: "Launcher hotkey",
        keywords: &["hotkey", "launcher", "shortcut", "global", "alt", "space"],
        focus_id: Some("launcher_hotkey_input"),
    },
    SearchEntry {
        page: Page::Launcher,
        section: "Hotkeys",
        label: "Omakase hotkey",
        keywords: &["hotkey", "omakase", "shortcut", "global", "system", "menu"],
        focus_id: Some("launcher_omakase_hotkey_input"),
    },
    SearchEntry {
        page: Page::Launcher,
        section: "Indexing",
        label: "Scan interval (minutes)",
        keywords: &["scan", "interval", "index", "start menu", "refresh", "minutes"],
        focus_id: Some("launcher_scan_interval"),
    },
    SearchEntry {
        page: Page::Launcher,
        section: "Indexing",
        label: "Extra directories",
        keywords: &["extra", "directories", "folders", "index", "paths", "start menu"],
        focus_id: Some("launcher_extra_dirs"),
    },
];

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let theme = app.cfg.theme;
    c::page_frame(ui, theme, |ui| {
        c::page_header(
            ui,
            theme,
            "Launcher",
            Some("Global hotkeys and Start Menu indexing."),
        );

        c::section(ui, theme, "Hotkeys", |ui| {
            c::hotkey_cheatsheet(ui, theme);
            ui.add_space(theme::tokens().space_sm);

            hotkey_row(
                ui,
                theme,
                app,
                "launcher_hotkey_input",
                "Launcher hotkey",
                Field::Launcher,
            );
            hotkey_row(
                ui,
                theme,
                app,
                "launcher_omakase_hotkey_input",
                "Omakase hotkey",
                Field::Omakase,
            );
        });

        c::section(ui, theme, "Indexing", |ui| {
            c::field_row(ui, theme, "Scan interval (minutes)", |ui| {
                let resp = ui.add(
                    egui::DragValue::new(&mut app.cfg.launcher.scan_interval_minutes)
                        .range(1..=120)
                        .speed(1.0),
                );
                c::consume_focus_target(
                    &resp,
                    &mut app.focus_target,
                    "launcher_scan_interval",
                );
            });

            c::stacked_field(ui, theme, "Extra directories", |ui| {
                extra_dirs(ui, theme, app);
            });
        });
    });
}

enum Field {
    Launcher,
    Omakase,
}

/// Renders one hotkey row using the shared `hotkey_input` widget. Commits the
/// canonical AHK form to the config the moment the user types a valid
/// combination; mid-typing invalid states stay only in the UI buffer so the
/// active launcher / omakase shortcut can't be broken by an in-progress edit.
fn hotkey_row(
    ui: &mut egui::Ui,
    theme: Theme,
    app: &mut App,
    focus_name: &'static str,
    label: &str,
    field: Field,
) {
    c::field_row(ui, theme, label, |ui| {
        let avail = ui.available_width();
        let buf = match field {
            Field::Launcher => &mut app.hotkey_input,
            Field::Omakase => &mut app.omakase_hotkey_input,
        };
        let result = c::hotkey_input(ui, theme, buf, avail, None);
        c::consume_focus_target(&result.response, &mut app.focus_target, focus_name);
        if let Some(spec) = result.spec {
            let normalized = spec.to_ahk();
            match field {
                Field::Launcher => {
                    if app.cfg.launcher.hotkey.0 != normalized {
                        app.cfg.launcher.hotkey.0 = normalized;
                    }
                }
                Field::Omakase => {
                    if app.cfg.launcher.omakase_hotkey.0 != normalized {
                        app.cfg.launcher.omakase_hotkey.0 = normalized;
                    }
                }
            }
        }
    });

    // Registration-time errors (reserved / OS-level conflicts) live under the
    // field, aligned with the input column.
    let err = match field {
        Field::Launcher => app.hotkey_error.clone(),
        Field::Omakase => app.omakase_hotkey_error.clone(),
    };
    if let Some(e) = err {
        indent_under_label(ui, |ui| {
            c::inline_error(ui, theme, &e);
        });
    }
}

fn extra_dirs(ui: &mut egui::Ui, _theme: Theme, app: &mut App) {
    let dirs = &mut app.cfg.launcher.extra_dirs;
    let mut remove: Option<usize> = None;
    for (i, dir) in dirs.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            let mut buf = dir.to_string_lossy().to_string();
            let prev = buf.clone();
            let avail = ui.available_width();
            ui.add_sized(
                [avail - 32.0, 24.0],
                egui::TextEdit::singleline(&mut buf),
            );
            if buf != prev {
                *dir = PathBuf::from(buf);
            }
            if ui.button("×").on_hover_text("remove").clicked() {
                remove = Some(i);
            }
        });
        ui.add_space(theme::tokens().space_xs);
    }
    if let Some(i) = remove {
        dirs.remove(i);
    }
    ui.add_space(theme::tokens().space_xs);
    if ui.button("+ Add directory").clicked() {
        dirs.push(PathBuf::new());
    }
}

/// Indent helper for error text that should align with the right-hand
/// column of a [`field_row`] (i.e. under the input, not under the label).
fn indent_under_label(ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui)) {
    let t = theme::tokens();
    ui.horizontal(|ui| {
        ui.add_space(t.field_label_width);
        ui.vertical(body);
    });
    ui.add_space(t.space_xs);
}
