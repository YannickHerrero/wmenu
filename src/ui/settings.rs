use eframe::egui::{self, Frame, Key, KeyboardShortcut, Margin, Modifiers, Ui};

use crate::config::{HotkeyBinding, Theme};
use crate::hotkey::BindingError;
use crate::ui::theme::Palette;

pub enum Action {
    None,
    Back,
    ThemeChanged(Theme),
    ApplyHotkey(String),
    AddBinding,
    RemoveBinding(usize),
    ApplyBindings,
    ToggleAutostart(bool),
}

const THEMES: [(Theme, &str); 5] = [
    (Theme::Paper, "Paper"),
    (Theme::Stone, "Stone"),
    (Theme::Sage, "Sage"),
    (Theme::Clay, "Clay"),
    (Theme::Ink, "Ink"),
];

const SAVE_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::S);
const ADD_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::N);

pub fn show(
    ui: &mut Ui,
    palette: &Palette,
    theme: &mut Theme,
    hotkey_input: &mut String,
    hotkey_error: Option<&str>,
    bindings: &mut [HotkeyBinding],
    binding_errors: &[BindingError],
    initial_focus_request: bool,
    focus_binding: Option<usize>,
    autostart_enabled: bool,
) -> Action {
    let mut action = Action::None;

    ui.input(|i| {
        if i.key_pressed(Key::Escape) {
            action = Action::Back;
        }
    });

    if ui.input_mut(|i| i.consume_shortcut(&SAVE_SHORTCUT)) {
        action = Action::ApplyBindings;
    }
    if ui.input_mut(|i| i.consume_shortcut(&ADD_SHORTCUT)) {
        action = Action::AddBinding;
    }

    egui::Frame::default()
        .fill(palette.paper)
        .inner_margin(Margin::symmetric(12, 10))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let back = ui.button("← Back");
                        if initial_focus_request {
                            back.request_focus();
                        }
                        if back.clicked() {
                            action = Action::Back;
                        }
                        ui.colored_label(palette.ink, "Settings");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.colored_label(
                                palette.ink_faint,
                                format!("v{}", env!("CARGO_PKG_VERSION")),
                            );
                        });
                    });
                    ui.add_space(8.0);

                    ui.colored_label(palette.ink_soft, "Theme");
                    for (variant, label) in THEMES {
                        let selected = *theme == variant;
                        if ui
                            .radio(selected, egui::RichText::new(label).color(palette.ink))
                            .clicked()
                            && !selected
                        {
                            *theme = variant;
                            action = Action::ThemeChanged(variant);
                        }
                    }

                    ui.add_space(12.0);
                    ui.colored_label(palette.ink_soft, "Launcher hotkey");
                    ui.horizontal(|ui| {
                        text_field(ui, palette, hotkey_input, 180.0);
                        if ui.button("Apply").clicked() {
                            action = Action::ApplyHotkey(hotkey_input.clone());
                        }
                    });
                    ui.colored_label(palette.ink_faint, "e.g. Alt+Space, Ctrl+Alt+Space");
                    if let Some(err) = hotkey_error {
                        ui.colored_label(palette.accent, format!("Error: {err}"));
                    }

                    ui.add_space(16.0);
                    ui.colored_label(palette.ink_soft, "Startup");
                    let mut as_state = autostart_enabled;
                    if ui
                        .checkbox(
                            &mut as_state,
                            egui::RichText::new("Launch wmenu at Windows login").color(palette.ink),
                        )
                        .changed()
                    {
                        action = Action::ToggleAutostart(as_state);
                    }

                    ui.add_space(16.0);
                    ui.colored_label(palette.ink_soft, "Hotkey bindings");
                    ui.colored_label(
                        palette.ink_faint,
                        "Label · Hotkey · Command. Ctrl+N add · Ctrl+S apply.",
                    );

                    for (idx, binding) in bindings.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            let label_resp = text_field(ui, palette, &mut binding.label, 110.0);
                            if focus_binding == Some(idx) {
                                label_resp.request_focus();
                            }
                            text_field(ui, palette, &mut binding.spec, 130.0);
                            text_field(ui, palette, &mut binding.command, 260.0);
                            if ui.button("Remove").clicked() {
                                action = Action::RemoveBinding(idx);
                            }
                        });
                        if let Some(err) = binding_errors.iter().find(|e| e.index == idx) {
                            ui.colored_label(palette.accent, format!("Error: {}", err.message));
                        }
                    }

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("+ Add").clicked() {
                            action = Action::AddBinding;
                        }
                        if ui.button("Apply").clicked() {
                            action = Action::ApplyBindings;
                        }
                    });
                });
        });

    action
}

fn text_field(ui: &mut Ui, palette: &Palette, text: &mut String, width: f32) -> egui::Response {
    let response = Frame::default()
        .fill(palette.muted)
        .inner_margin(Margin::symmetric(6, 4))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(text)
                    .desired_width(width)
                    .text_color(palette.ink)
                    .frame(Frame::NONE),
            )
        });
    response.inner
}
