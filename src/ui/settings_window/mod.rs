pub mod bindings;
pub mod general;
pub mod launcher;

use eframe::egui;

use crate::app::App;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    General,
    Launcher,
    Bindings,
    Amphetamine,
    About,
}

pub fn render(app: &mut App, child_ctx: &egui::Context) {
    #[allow(deprecated)]
    egui::CentralPanel::default().show(child_ctx, |ui| {
        egui::Panel::left("settings_nav")
            .resizable(false)
            .default_size(180.0)
            .show_inside(ui, |ui| {
                ui.add_space(12.0);
                ui.heading("wmenu");
                ui.add_space(16.0);
                nav_button(ui, &mut app.settings_page, Page::General, "General");
                nav_button(ui, &mut app.settings_page, Page::Launcher, "Launcher");
                nav_button(ui, &mut app.settings_page, Page::Bindings, "Bindings");
                nav_button(
                    ui,
                    &mut app.settings_page,
                    Page::Amphetamine,
                    "Amphetamine",
                );
                nav_button(ui, &mut app.settings_page, Page::About, "About");
            });

        egui::Panel::bottom("settings_status").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    save_and_apply(app, child_ctx);
                }
                if app.settings_dirty {
                    ui.label(
                        egui::RichText::new("• unsaved changes")
                            .weak()
                            .small(),
                    );
                }
                if let Some(s) = &app.settings_status {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(s).small().weak());
                    });
                }
            });
        });

        let before = config_signature(&app.cfg);
        match app.settings_page {
            Page::General => general::show(app, ui),
            Page::Launcher => launcher::show(app, ui),
            Page::Bindings => bindings::show(app, ui),
            Page::Amphetamine => stub(ui, "Amphetamine"),
            Page::About => stub(ui, "About"),
        }
        if config_signature(&app.cfg) != before {
            app.settings_dirty = true;
        }
    });
}

fn nav_button(ui: &mut egui::Ui, current: &mut Page, target: Page, label: &str) {
    let selected = *current == target;
    if ui.selectable_label(selected, label).clicked() {
        *current = target;
    }
}

fn stub(ui: &mut egui::Ui, name: &str) {
    ui.heading(name);
    ui.add_space(8.0);
    ui.label("(under construction)");
}

fn config_signature(cfg: &crate::config::Config) -> Option<String> {
    toml::to_string(cfg).ok()
}

fn save_and_apply(app: &mut App, child_ctx: &egui::Context) {
    match app.cfg.save() {
        Ok(()) => {
            app.settings_dirty = false;
            app.settings_status = Some(format!(
                "Saved to {}",
                crate::config::config_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "(unknown)".into())
            ));
        }
        Err(e) => {
            app.settings_status = Some(format!("Save failed: {e}"));
            tracing::warn!("save config: {e}");
            return;
        }
    }
    app.apply_reloaded(child_ctx);
}
