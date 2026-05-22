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

/// A single searchable setting. Pages export a static slice of these so the
/// top-bar search input can offer "jump to this field" results.
#[allow(dead_code)] // consumed by the search-results view in the next commit
#[derive(Debug, Clone, Copy)]
pub struct SearchEntry {
    pub page: Page,
    pub section: &'static str,
    pub label: &'static str,
    pub keywords: &'static [&'static str],
    /// Optional widget id the focus hand-off should attempt to focus after
    /// jumping to the target page. Pages create the matching id via
    /// `egui::Id::new(focus_id)`.
    pub focus_id: Option<&'static str>,
}

/// All search entries from every page, concatenated. Bindings are dynamic so
/// they're handled separately at search time.
#[allow(dead_code)] // consumed by the search-results view in the next commit
pub fn static_entries() -> Vec<SearchEntry> {
    let mut v = Vec::new();
    v.extend_from_slice(general::ENTRIES);
    v.extend_from_slice(launcher::ENTRIES);
    v.extend_from_slice(bindings::ENTRIES);
    v.extend_from_slice(amphetamine::ENTRIES);
    v.extend_from_slice(about::ENTRIES);
    v
}

pub fn render(app: &mut App, child_ctx: &egui::Context) {
    let t = theme::tokens();
    let p = theme::palette(app.cfg.theme);

    // Global shortcuts that work even while a field is focused.
    if child_ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::F)) {
        app.settings_search_focus_request = true;
    }

    handle_sidebar_keys(app, child_ctx);

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

                                        // Search input fills the remaining
                                        // horizontal slack between the title
                                        // and the right-side controls.
                                        ui.add_space(t.space_md);
                                        search_input(ui, app, child_ctx);
                                    },
                                );
                            });
                            ui.add_space(t.space_sm);
                            components::hairline(ui, p.ink_faint);
                        });
                });

            egui::Panel::left("settings_nav")
                .resizable(false)
                .default_size(192.0)
                .show_inside(ui, |ui| {
                    let nav_pad = egui::Margin {
                        left: t.space_sm as i8,
                        right: t.space_sm as i8,
                        top: t.space_md as i8,
                        bottom: t.space_md as i8,
                    };
                    egui::Frame::new()
                        .fill(p.paper)
                        .inner_margin(nav_pad)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            for (page, label) in PAGES {
                                nav_button(ui, app, *page, label);
                            }
                        });
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

/// The search input in the top header. Stretches to fill the remaining
/// horizontal slack inside its right-to-left layout. Honours the
/// `settings_search_focus_request` flag set by `Ctrl+F`.
fn search_input(ui: &mut egui::Ui, app: &mut App, ctx: &egui::Context) {
    let t = theme::tokens();
    let id = egui::Id::new("settings_search_input");
    let avail = ui.available_width();
    let width = avail.clamp(160.0, 360.0);
    let resp = ui.add_sized(
        [width, 28.0],
        egui::TextEdit::singleline(&mut app.settings_search)
            .hint_text("Search settings…  Ctrl+F")
            .id(id),
    );
    if app.settings_search_focus_request {
        resp.request_focus();
        app.settings_search_focus_request = false;
    }
    // Esc clears the search when the search input itself is focused.
    let focused = ctx.memory(|m| m.focused()) == Some(id);
    if focused
        && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        app.settings_search.clear();
    }
    let _ = t;
}

/// Arrow / j / k navigation across the sidebar. Only fires when no text-input
/// widget is currently capturing keystrokes, so typing inside a field is
/// unaffected.
fn handle_sidebar_keys(app: &mut App, ctx: &egui::Context) {
    if ctx.egui_wants_keyboard_input() {
        return;
    }
    let len = PAGES.len();
    let current = PAGES.iter().position(|(p, _)| *p == app.settings_page).unwrap_or(0);
    let mut next = current;
    ctx.input_mut(|i| {
        if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
            || i.consume_key(egui::Modifiers::NONE, egui::Key::J)
        {
            next = (current + 1) % len;
        }
        if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)
            || i.consume_key(egui::Modifiers::NONE, egui::Key::K)
        {
            next = (current + len - 1) % len;
        }
        if i.consume_key(egui::Modifiers::NONE, egui::Key::Home) {
            next = 0;
        }
        if i.consume_key(egui::Modifiers::NONE, egui::Key::End) {
            next = len - 1;
        }
    });
    if next != current {
        app.settings_page = PAGES[next].0;
    }
}

pub const PAGES: &[(Page, &str)] = &[
    (Page::General, "General"),
    (Page::Launcher, "Launcher"),
    (Page::Bindings, "Bindings"),
    (Page::Amphetamine, "Amphetamine"),
    (Page::About, "About"),
];

fn nav_button(ui: &mut egui::Ui, app: &mut App, target: Page, label: &str) {
    let t = theme::tokens();
    let p = theme::palette(app.cfg.theme);
    let selected = app.settings_page == target;

    let row_h = 32.0;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), row_h),
        egui::Sense::click(),
    );

    let hovered = response.hovered();
    let bg = if selected {
        p.accent
    } else if hovered {
        p.muted
    } else {
        p.paper
    };
    let fg = if selected { p.paper } else { p.ink };

    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::same(t.radius_sm as u8),
        bg,
    );

    let text_x = rect.left() + t.space_md;
    ui.painter().text(
        egui::pos2(text_x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(t.font_body),
        fg,
    );

    if response.clicked() {
        app.settings_page = target;
    }

    ui.add_space(t.space_xs);
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
