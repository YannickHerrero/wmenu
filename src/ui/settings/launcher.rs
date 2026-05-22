use std::path::PathBuf;

use eframe::egui;

use crate::app::App;
use crate::config::Theme;
use crate::ui::settings::components as c;
use crate::ui::settings::{Page, SearchEntry};
use crate::ui::theme;

#[allow(dead_code)] // consumed by the search-results view in the next commit
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
            if hotkey_row(
                ui,
                theme,
                "Launcher hotkey",
                &mut app.hotkey_input,
                app.hotkey_error.as_deref(),
                "e.g. Alt+Space, Ctrl+Alt+Space",
            ) {
                pending_apply(app, ApplyHotkey::Launcher);
            }
            if hotkey_row(
                ui,
                theme,
                "Omakase hotkey",
                &mut app.omakase_hotkey_input,
                app.omakase_hotkey_error.as_deref(),
                "e.g. Alt+Super+Space (Super = Win key)",
            ) {
                pending_apply(app, ApplyHotkey::Omakase);
            }
        });

        c::section(ui, theme, "Indexing", |ui| {
            c::field_row(ui, theme, "Scan interval (minutes)", |ui| {
                let prev = app.cfg.launcher.scan_interval_minutes;
                ui.add(
                    egui::DragValue::new(&mut app.cfg.launcher.scan_interval_minutes)
                        .range(1..=120)
                        .speed(1.0),
                );
                if app.cfg.launcher.scan_interval_minutes != prev {
                    app.settings_dirty = true;
                }
            });

            c::stacked_field(ui, theme, "Extra directories", |ui| {
                extra_dirs(ui, theme, app);
            });
        });
    });
}

enum ApplyHotkey {
    Launcher,
    Omakase,
}

/// Returns `true` if the user clicked the row's Apply button.
fn hotkey_row(
    ui: &mut egui::Ui,
    theme: Theme,
    label: &str,
    buf: &mut String,
    err: Option<&str>,
    hint: &str,
) -> bool {
    let mut applied = false;
    c::field_row(ui, theme, label, |ui| {
        let apply_btn_w = 60.0;
        let avail = ui.available_width();
        ui.add_sized(
            [avail - apply_btn_w - 8.0, 24.0],
            egui::TextEdit::singleline(buf),
        );
        if ui
            .add_sized([apply_btn_w, 24.0], egui::Button::new("Apply"))
            .clicked()
        {
            applied = true;
        }
    });
    indent_under_label(ui, |ui| {
        ui.label(
            egui::RichText::new(hint)
                .small()
                .color(theme::palette(theme).ink_soft),
        );
    });
    if let Some(e) = err {
        indent_under_label(ui, |ui| {
            c::inline_error(ui, theme, e);
        });
    }
    applied
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
                app.settings_dirty = true;
            }
            if ui.button("×").on_hover_text("remove").clicked() {
                remove = Some(i);
            }
        });
        ui.add_space(theme::tokens().space_xs);
    }
    if let Some(i) = remove {
        dirs.remove(i);
        app.settings_dirty = true;
    }
    ui.add_space(theme::tokens().space_xs);
    if ui.button("+ Add directory").clicked() {
        dirs.push(PathBuf::new());
        app.settings_dirty = true;
    }
}

/// Indent helper for hint / error text that should align with the right-hand
/// column of a [`field_row`] (i.e. under the input, not under the label).
fn indent_under_label(ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui)) {
    let t = theme::tokens();
    ui.horizontal(|ui| {
        ui.add_space(t.field_label_width);
        ui.vertical(body);
    });
    ui.add_space(t.space_xs);
}

fn pending_apply(app: &mut App, which: ApplyHotkey) {
    match which {
        ApplyHotkey::Launcher => {
            let spec = app.hotkey_input.clone();
            match app.hotkey.set(&spec) {
                Ok(_) => {
                    app.cfg.launcher.hotkey.0 = spec;
                    app.hotkey_error = None;
                    app.settings_dirty = true;
                }
                Err(e) => app.hotkey_error = Some(format!("{e}")),
            }
        }
        ApplyHotkey::Omakase => {
            let spec = app.omakase_hotkey_input.clone();
            match app.hotkey.set_omakase(&spec) {
                Ok(_) => {
                    app.cfg.launcher.omakase_hotkey.0 = spec;
                    app.omakase_hotkey_error = None;
                    app.settings_dirty = true;
                }
                Err(e) => app.omakase_hotkey_error = Some(format!("{e}")),
            }
        }
    }
}
