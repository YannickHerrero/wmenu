use std::path::PathBuf;

use eframe::egui;

use crate::app::App;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("Launcher");
    ui.add_space(8.0);

    section(ui, "Hotkeys", |ui| {
        if let Some(a) = hotkey_row(
            ui,
            "Launcher",
            &mut app.hotkey_input,
            app.hotkey_error.as_deref(),
            "e.g. Alt+Space, Ctrl+Alt+Space",
            || ApplyHotkey::Launcher,
        ) {
            pending_apply(app, a);
        }
        ui.add_space(6.0);
        if let Some(a) = hotkey_row(
            ui,
            "Omakase",
            &mut app.omakase_hotkey_input,
            app.omakase_hotkey_error.as_deref(),
            "e.g. Alt+Super+Space (Super = Win key)",
            || ApplyHotkey::Omakase,
        ) {
            pending_apply(app, a);
        }
    });

    ui.add_space(12.0);

    section(ui, "Indexing", |ui| {
        ui.horizontal(|ui| {
            ui.label("Scan interval (minutes)");
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
        ui.add_space(6.0);
        ui.label("Extra directories");
        let dirs = &mut app.cfg.launcher.extra_dirs;
        let mut remove: Option<usize> = None;
        for (i, dir) in dirs.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                let mut buf = dir.to_string_lossy().to_string();
                let prev = buf.clone();
                ui.add(egui::TextEdit::singleline(&mut buf).desired_width(320.0));
                if buf != prev {
                    *dir = PathBuf::from(buf);
                    app.settings_dirty = true;
                }
                if ui.button("×").on_hover_text("remove").clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            dirs.remove(i);
            app.settings_dirty = true;
        }
        if ui.button("+ Add directory").clicked() {
            dirs.push(PathBuf::new());
            app.settings_dirty = true;
        }
    });
}

enum ApplyHotkey {
    Launcher,
    Omakase,
}

fn hotkey_row(
    ui: &mut egui::Ui,
    label: &str,
    buf: &mut String,
    err: Option<&str>,
    hint: &str,
    apply: impl Fn() -> ApplyHotkey,
) -> Option<ApplyHotkey> {
    let mut result = None;
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::TextEdit::singleline(buf).desired_width(200.0));
        if ui.button("Apply").clicked() {
            result = Some(apply());
        }
    });
    ui.label(egui::RichText::new(hint).small().weak());
    if let Some(e) = err {
        ui.colored_label(egui::Color32::RED, format!("Error: {e}"));
    }
    result
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

fn section(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    ui.label(egui::RichText::new(title).strong());
    ui.add_space(4.0);
    ui.indent(title, body);
}
