use eframe::egui::{self, Frame, Key, Margin, Ui};

use crate::config::Theme;
use crate::ui::theme::Palette;

pub enum Action {
    None,
    Back,
    ThemeChanged(Theme),
    ApplyHotkey(String),
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
            ui.horizontal(|ui| {
                if ui.button("← Back").clicked() {
                    action = Action::Back;
                }
                ui.colored_label(palette.ink, "Settings");
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
            ui.colored_label(palette.ink_soft, "Global hotkey");
            ui.horizontal(|ui| {
                Frame::default()
                    .fill(palette.muted)
                    .inner_margin(Margin::symmetric(6, 4))
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(hotkey_input)
                                .desired_width(180.0)
                                .text_color(palette.ink)
                                .frame(Frame::NONE),
                        );
                    });
                if ui.button("Apply").clicked() {
                    action = Action::ApplyHotkey(hotkey_input.clone());
                }
            });
            ui.colored_label(
                palette.ink_faint,
                "e.g. Shift+Space, Ctrl+Alt+Space, Super+P",
            );
            if let Some(err) = hotkey_error {
                ui.colored_label(palette.accent, format!("Error: {err}"));
            }
        });

    action
}
