use eframe::egui::{self, Frame, Key, Margin, Ui};

use crate::config::Theme;
use crate::ui::theme::Palette;

pub enum Action {
    None,
    Back,
    ThemeChanged(Theme),
    ApplyHotkey(String),
    ApplyOmakaseHotkey(String),
    ToggleAutostart(bool),
}

const THEMES: [(Theme, &str); 5] = [
    (Theme::Paper, "Paper"),
    (Theme::Stone, "Stone"),
    (Theme::Sage, "Sage"),
    (Theme::Clay, "Clay"),
    (Theme::Ink, "Ink"),
];

pub fn show(
    ui: &mut Ui,
    palette: &Palette,
    theme: &mut Theme,
    hotkey_input: &mut String,
    hotkey_error: Option<&str>,
    omakase_hotkey_input: &mut String,
    omakase_hotkey_error: Option<&str>,
    initial_focus_request: bool,
    autostart_enabled: bool,
) -> Action {
    let mut action = Action::None;

    ui.input(|i| {
        if i.key_pressed(Key::Escape) {
            action = Action::Back;
        }
    });

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

                    ui.add_space(12.0);
                    ui.colored_label(palette.ink_soft, "Omakase hotkey");
                    ui.horizontal(|ui| {
                        text_field(ui, palette, omakase_hotkey_input, 180.0);
                        if ui.button("Apply").clicked() {
                            action = Action::ApplyOmakaseHotkey(omakase_hotkey_input.clone());
                        }
                    });
                    ui.colored_label(palette.ink_faint, "e.g. Alt+Super+Space (Super = Win key)");
                    if let Some(err) = omakase_hotkey_error {
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
