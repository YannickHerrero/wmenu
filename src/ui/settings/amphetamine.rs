use eframe::egui;

use crate::app::App;
use crate::ui::settings::components as c;
use crate::ui::theme;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let theme = app.cfg.theme;
    c::page_frame(ui, theme, |ui| {
        c::page_header(
            ui,
            theme,
            "Amphetamine",
            Some(
                "Keep Windows from dimming the screen or starting the screensaver while \
                 you're away.",
            ),
        );

        c::section(ui, theme, "Cursor wiggle", |ui| {
            c::field_row(ui, theme, "Keep Windows awake", |ui| {
                let prev = app.cfg.amphetamine_enabled;
                ui.checkbox(&mut app.cfg.amphetamine_enabled, "");
                if app.cfg.amphetamine_enabled != prev {
                    app.amphetamine.set(app.cfg.amphetamine_enabled);
                }
                if app.cfg.amphetamine_enabled {
                    active_dot(ui, theme);
                    ui.label(
                        egui::RichText::new("Active — cursor nudges every 4 minutes")
                            .small()
                            .color(theme::palette(theme).ink_soft),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("Idle")
                            .small()
                            .color(theme::palette(theme).ink_faint),
                    );
                }
            });
        });
    });
}

fn active_dot(ui: &mut egui::Ui, theme: crate::config::Theme) {
    let t = theme::tokens();
    let p = theme::palette(theme);
    let size = egui::vec2(t.space_sm, t.space_sm);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), size.x * 0.5, p.success);
}
