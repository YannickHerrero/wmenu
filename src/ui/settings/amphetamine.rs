use eframe::egui;

use crate::app::App;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("Amphetamine");
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(
            "Wiggles the cursor by a few pixels every 4 minutes to keep Windows from \
             dimming the screen or starting the screensaver.",
        )
        .small()
        .weak(),
    );
    ui.add_space(12.0);

    let prev = app.cfg.amphetamine_enabled;
    ui.checkbox(&mut app.cfg.amphetamine_enabled, "Keep Windows awake");
    if app.cfg.amphetamine_enabled != prev {
        app.amphetamine.set(app.cfg.amphetamine_enabled);
    }
}
