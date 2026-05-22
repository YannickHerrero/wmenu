pub mod about;
pub mod amphetamine;
pub mod bindings;
pub mod components;
pub mod general;
pub mod launcher;

use eframe::egui;

use crate::app::App;
use crate::ui::theme;

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
    let t = theme::tokens();
    let p = theme::palette(app.cfg.theme);

    #[allow(deprecated)]
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(p.paper))
        .show(child_ctx, |ui| {
            egui::Panel::top("settings_header")
                .resizable(false)
                .default_size(64.0)
                .show_inside(ui, |ui| {
                    let header_pad = egui::Margin {
                        left: t.space_lg as i8,
                        right: t.space_lg as i8,
                        top: t.space_md as i8,
                        bottom: t.space_md as i8,
                    };
                    egui::Frame::new()
                        .fill(p.paper)
                        .inner_margin(header_pad)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Settings")
                                        .size(t.font_page_title)
                                        .color(p.ink)
                                        .strong(),
                                );

                                // Right-aligned: status pill area + save button.
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.button("Save").clicked() {
                                            save_and_apply(app, child_ctx);
                                        }
                                        if app.settings_dirty {
                                            ui.label(
                                                egui::RichText::new("• unsaved")
                                                    .small()
                                                    .color(p.ink_soft),
                                            );
                                        }
                                        if let Some(s) = &app.settings_status {
                                            ui.label(
                                                egui::RichText::new(s)
                                                    .small()
                                                    .color(p.ink_soft),
                                            );
                                        }
                                    },
                                );
                            });
                            ui.add_space(t.space_sm);
                            components::hairline(ui, p.ink_faint);
                        });
                });

            egui::Panel::left("settings_nav")
                .resizable(false)
                .default_size(180.0)
                .show_inside(ui, |ui| {
                    ui.add_space(t.space_md);
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

            let before = config_signature(&app.cfg);
            match app.settings_page {
                Page::General => general::show(app, ui),
                Page::Launcher => launcher::show(app, ui),
                Page::Bindings => bindings::show(app, ui),
                Page::Amphetamine => amphetamine::show(app, ui),
                Page::About => about::show(app, ui),
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
