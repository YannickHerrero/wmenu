use eframe::egui::{self, Key, Margin, Ui};

use crate::config::Theme;
use crate::ui::theme::Palette;

pub enum Action {
    None,
    Back,
    ThemeChanged(Theme),
}

const THEMES: [(Theme, &str); 5] = [
    (Theme::Paper, "Paper"),
    (Theme::Stone, "Stone"),
    (Theme::Sage, "Sage"),
    (Theme::Clay, "Clay"),
    (Theme::Ink, "Ink"),
];

pub fn show(ui: &mut Ui, palette: &Palette, theme: &mut Theme) -> Action {
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
            ui.colored_label(palette.ink_faint, "Hotkey rebind: (coming in step 21)");
        });

    action
}
