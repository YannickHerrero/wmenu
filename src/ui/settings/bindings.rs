use eframe::egui;

use crate::app::App;
use crate::config::{Action, Binding, ShellKind, Theme};
use crate::ui::settings::components as c;
use crate::ui::settings::{Page, SearchEntry};
use crate::ui::theme;

/// Focus id of the first widget the cross-zone "→ / l" jump should land on.
pub const FIRST_FOCUS: Option<&str> = Some("bindings_add");

/// Static jump-to-page entry. Individual user bindings are matched at search
/// time by reading `app.cfg.bindings` directly, since they're dynamic.
pub const ENTRIES: &[SearchEntry] = &[SearchEntry {
    page: Page::Bindings,
    section: "Bindings",
    label: "Bindings",
    keywords: &["binding", "hotkey", "shortcut", "key", "action", "launch", "url", "script"],
    focus_id: None,
}];

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let theme = app.cfg.theme;
    c::page_frame(ui, theme, |ui| {
        c::page_header(
            ui,
            theme,
            "Bindings",
            Some(
                "Map global key combinations to commands, URLs, scripts, or window focus.",
            ),
        );

        // Toolbar: + Add binding on the right.
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let resp = ui.button("+ Add binding");
                c::consume_focus_target(&resp, &mut app.focus_target, "bindings_add");
                if resp.clicked() {
                    app.cfg.bindings.push(Binding {
                        label: String::from("New binding"),
                        key: String::from("Ctrl+Alt+N"),
                        action: Action::Launch {
                            command: String::new(),
                        },
                    });
                }
            });
        });
        ui.add_space(theme::tokens().space_sm);

        let mut remove: Option<usize> = None;
        let errors = app.binding_errors.clone();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .show(ui, |ui| {
                for (idx, binding) in app.cfg.bindings.iter_mut().enumerate() {
                    ui.push_id(idx, |ui| {
                        c::card(ui, theme, |ui| {
                            binding_header(ui, theme, binding, &mut remove, idx);
                            ui.add_space(theme::tokens().space_sm);
                            action_editor(ui, theme, &mut binding.action);
                            for err in errors.iter().filter(|e| e.index == idx) {
                                ui.add_space(theme::tokens().space_xs);
                                c::inline_error(ui, theme, &err.message);
                            }
                        });
                    });
                    ui.add_space(theme::tokens().space_sm);
                }
            });

        if let Some(idx) = remove {
            app.cfg.bindings.remove(idx);
        }
    });
}

fn binding_header(
    ui: &mut egui::Ui,
    theme: Theme,
    binding: &mut Binding,
    remove: &mut Option<usize>,
    idx: usize,
) {
    let t = theme::tokens();
    let p = theme::palette(theme);
    ui.horizontal(|ui| {
        // Label spans the left half so long binding names stay readable.
        let avail = ui.available_width();
        let remove_w = 80.0;
        let key_w = 160.0;
        let label_w = (avail - key_w - remove_w - 2.0 * t.space_sm).max(120.0);

        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("Label")
                    .small()
                    .color(p.ink_soft),
            );
            ui.add_sized(
                [label_w, 24.0],
                egui::TextEdit::singleline(&mut binding.label),
            );
        });

        ui.add_space(t.space_sm);

        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("Key combo")
                    .small()
                    .color(p.ink_soft),
            );
            ui.add_sized(
                [key_w, 24.0],
                egui::TextEdit::singleline(&mut binding.key),
            );
        });

        ui.add_space(t.space_sm);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
            // Match the input row height by padding above the button.
            ui.vertical(|ui| {
                ui.add_space(t.font_body + t.space_xs);
                if ui.add_sized([remove_w, 24.0], egui::Button::new("Remove")).clicked() {
                    *remove = Some(idx);
                }
            });
        });
    });
}

fn action_editor(ui: &mut egui::Ui, theme: Theme, action: &mut Action) {
    let p = theme::palette(theme);
    let t = theme::tokens();

    ui.label(
        egui::RichText::new("ACTION")
            .small()
            .color(p.ink_soft)
            .strong(),
    );
    ui.add_space(t.space_xs);

    c::field_row(ui, theme, "Type", |ui| {
        let label = action_type_label(action);
        egui::ComboBox::from_id_salt("action_type")
            .selected_text(label)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(matches!(action, Action::Launch { .. }), "Launch")
                    .clicked()
                {
                    *action = Action::Launch {
                        command: take_command(action),
                    };
                }
                if ui
                    .selectable_label(matches!(action, Action::Url { .. }), "Open URL")
                    .clicked()
                {
                    *action = Action::Url { url: String::new() };
                }
                if ui
                    .selectable_label(matches!(action, Action::Script { .. }), "Run script")
                    .clicked()
                {
                    *action = Action::Script {
                        shell: ShellKind::Powershell,
                        script: String::new(),
                    };
                }
                if ui
                    .selectable_label(
                        matches!(action, Action::FocusOrLaunch { .. }),
                        "Focus or launch",
                    )
                    .clicked()
                {
                    *action = Action::FocusOrLaunch {
                        exe_path: String::new(),
                        match_basename: true,
                        launch_args: Vec::new(),
                    };
                }
            });
    });

    match action {
        Action::Launch { command } => {
            c::field_row(ui, theme, "Command", |ui| {
                ui.add(egui::TextEdit::singleline(command).desired_width(f32::INFINITY));
            });
        }
        Action::Url { url } => {
            c::field_row(ui, theme, "URL", |ui| {
                ui.add(egui::TextEdit::singleline(url).desired_width(f32::INFINITY));
            });
        }
        Action::Script { shell, script } => {
            c::field_row(ui, theme, "Shell", |ui| {
                egui::ComboBox::from_id_salt("shell_kind")
                    .selected_text(shell_label(*shell))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(shell, ShellKind::Powershell, "PowerShell");
                        ui.selectable_value(shell, ShellKind::Cmd, "cmd");
                        ui.selectable_value(shell, ShellKind::Pwsh, "pwsh (PowerShell 7+)");
                    });
            });
            c::stacked_field(ui, theme, "Script", |ui| {
                ui.add(
                    egui::TextEdit::multiline(script)
                        .desired_width(f32::INFINITY)
                        .desired_rows(4)
                        .code_editor(),
                );
            });
        }
        Action::FocusOrLaunch {
            exe_path,
            match_basename,
            launch_args,
        } => {
            c::field_row(ui, theme, "Exe path", |ui| {
                ui.add(egui::TextEdit::singleline(exe_path).desired_width(f32::INFINITY));
            });
            c::field_row(ui, theme, "Match basename only", |ui| {
                ui.checkbox(match_basename, "");
                ui.label(
                    egui::RichText::new("e.g. firefox.exe, instead of the full path")
                        .small()
                        .color(p.ink_soft),
                );
            });
            c::stacked_field(ui, theme, "Launch args (one per line)", |ui| {
                let mut joined = launch_args.join("\n");
                let resp = ui.add(
                    egui::TextEdit::multiline(&mut joined)
                        .desired_width(f32::INFINITY)
                        .desired_rows(2),
                );
                if resp.changed() {
                    *launch_args = joined
                        .lines()
                        .map(|l| l.to_string())
                        .filter(|l| !l.is_empty())
                        .collect();
                }
            });
        }
    }
}

fn action_type_label(action: &Action) -> &'static str {
    match action {
        Action::Launch { .. } => "Launch",
        Action::Url { .. } => "Open URL",
        Action::Script { .. } => "Run script",
        Action::FocusOrLaunch { .. } => "Focus or launch",
    }
}

fn shell_label(s: ShellKind) -> &'static str {
    match s {
        ShellKind::Powershell => "PowerShell",
        ShellKind::Cmd => "cmd",
        ShellKind::Pwsh => "pwsh",
    }
}

fn take_command(action: &Action) -> String {
    match action {
        Action::Launch { command } => command.clone(),
        Action::Url { url } => url.clone(),
        Action::Script { script, .. } => script.clone(),
        Action::FocusOrLaunch { exe_path, .. } => exe_path.clone(),
    }
}
