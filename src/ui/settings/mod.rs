pub mod about;
pub mod amphetamine;
pub mod bindings;
pub mod components;
pub mod general;
pub mod launcher;

use eframe::egui;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

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
pub fn static_entries() -> Vec<SearchEntry> {
    let mut v = Vec::new();
    v.extend_from_slice(general::ENTRIES);
    v.extend_from_slice(launcher::ENTRIES);
    v.extend_from_slice(bindings::ENTRIES);
    v.extend_from_slice(amphetamine::ENTRIES);
    v.extend_from_slice(about::ENTRIES);
    v
}

/// A single search hit ready for rendering. Wraps a [`SearchEntry`] with the
/// score (for sorting) and an optional dynamic detail (used for user
/// bindings whose label isn't known until config is loaded).
struct SearchHit {
    page: Page,
    section: String,
    label: String,
    focus_id: Option<&'static str>,
    score: u32,
}

fn search_settings(query: &str, app: &App) -> Vec<SearchHit> {
    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut buf = Vec::new();
    let mut hits: Vec<SearchHit> = Vec::new();

    for entry in static_entries() {
        // Concatenate label + keywords into one haystack so a match on any of
        // them surfaces the entry.
        let haystack = std::iter::once(entry.label)
            .chain(entry.keywords.iter().copied())
            .collect::<Vec<_>>()
            .join(" ");
        if let Some(score) = pattern.score(Utf32Str::new(&haystack, &mut buf), &mut matcher) {
            hits.push(SearchHit {
                page: entry.page,
                section: entry.section.to_string(),
                label: entry.label.to_string(),
                focus_id: entry.focus_id,
                score,
            });
        }
    }

    // Dynamic per-binding entries: surface each user binding by its label so
    // typing the binding name jumps straight to the page.
    for binding in &app.cfg.bindings {
        let haystack = format!("{} {} binding hotkey", binding.label, binding.key);
        if let Some(score) = pattern.score(Utf32Str::new(&haystack, &mut buf), &mut matcher) {
            hits.push(SearchHit {
                page: Page::Bindings,
                section: "Bindings".to_string(),
                label: format!("{} ({})", binding.label, binding.key),
                focus_id: None,
                score,
            });
        }
    }

    hits.sort_by(|a, b| b.score.cmp(&a.score));
    hits.truncate(40);
    hits
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

                                // Right-aligned: search box + save-status pill.
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        save_pill(ui, app);
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
            if app.settings_search.trim().is_empty() {
                match app.settings_page {
                    Page::General => general::show(app, ui),
                    Page::Launcher => launcher::show(app, ui),
                    Page::Bindings => bindings::show(app, ui),
                    Page::Amphetamine => amphetamine::show(app, ui),
                    Page::About => about::show(app, ui),
                }
            } else {
                search_results_view(ui, app);
            }
            if config_signature(&app.cfg) != before {
                app.last_edit_at = Some(std::time::Instant::now());
            }
        });

    auto_save_tick(app, child_ctx);
}

const SAVE_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(400);
const SAVED_PILL_LINGER: std::time::Duration = std::time::Duration::from_millis(1500);

/// Once per frame, decide whether the debounce window has elapsed and we
/// should flush the in-memory config to disk. Drives the "Saved" pill fade
/// via `request_repaint_after` so the UI updates without user input.
fn auto_save_tick(app: &mut App, ctx: &egui::Context) {
    if let Some(t) = app.last_edit_at {
        let elapsed = t.elapsed();
        if elapsed >= SAVE_DEBOUNCE {
            match app.save_config() {
                Ok(()) => {
                    app.last_edit_at = None;
                    app.last_saved_at = Some(std::time::Instant::now());
                    app.last_save_error = None;
                    app.apply_reloaded(ctx);
                    ctx.request_repaint_after(SAVED_PILL_LINGER);
                }
                Err(e) => {
                    tracing::warn!("auto-save: {e}");
                    app.last_edit_at = None;
                    app.last_save_error = Some(e.to_string());
                }
            }
        } else {
            ctx.request_repaint_after(SAVE_DEBOUNCE - elapsed + std::time::Duration::from_millis(20));
        }
    }

    if let Some(at) = app.last_saved_at {
        let elapsed = at.elapsed();
        if elapsed >= SAVED_PILL_LINGER {
            app.last_saved_at = None;
        } else {
            ctx.request_repaint_after(SAVED_PILL_LINGER - elapsed);
        }
    }
}

fn save_pill(ui: &mut egui::Ui, app: &App) {
    use components::Kind;
    let theme = app.cfg.theme;
    if let Some(err) = &app.last_save_error {
        components::pill(ui, theme, Kind::Error, &format!("⚠ {err}"));
    } else if app.last_edit_at.is_some() {
        components::pill(ui, theme, Kind::Neutral, "Saving…");
    } else if app.last_saved_at.is_some() {
        components::pill(ui, theme, Kind::Success, "Saved");
    } else {
        // Nothing to show; emit an invisible spacer so the layout stays
        // stable as the pill appears and disappears.
        ui.add_space(0.0);
    }
}

/// Renders the search results list (page breadcrumb + label, grouped order
/// preserved from the matcher's score). Clicking a row jumps to the
/// matching page and clears the search.
fn search_results_view(ui: &mut egui::Ui, app: &mut App) {
    let theme = app.cfg.theme;
    let t = theme::tokens();
    let p = theme::palette(theme);
    let hits = search_settings(&app.settings_search, app);

    components::page_frame(ui, theme, |ui| {
        components::page_header(
            ui,
            theme,
            "Search",
            Some(&format!(
                "{} match{} for \"{}\"",
                hits.len(),
                if hits.len() == 1 { "" } else { "es" },
                app.settings_search
            )),
        );

        let mut jump: Option<(Page, Option<&'static str>)> = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .show(ui, |ui| {
                if hits.is_empty() {
                    ui.label(
                        egui::RichText::new("No settings match.")
                            .color(p.ink_soft)
                            .size(t.font_body),
                    );
                    return;
                }
                for hit in &hits {
                    let row_h = 40.0;
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), row_h),
                        egui::Sense::click(),
                    );
                    let hovered = response.hovered();
                    let bg = if hovered { p.muted } else { p.paper };
                    ui.painter().rect_filled(
                        rect,
                        egui::CornerRadius::same(t.radius_sm as u8),
                        bg,
                    );
                    let breadcrumb =
                        format!("{} › {}", page_label(hit.page), hit.section);
                    ui.painter().text(
                        egui::pos2(rect.left() + t.space_md, rect.top() + t.space_sm),
                        egui::Align2::LEFT_TOP,
                        &breadcrumb,
                        egui::FontId::proportional(t.font_section_title),
                        p.ink_soft,
                    );
                    ui.painter().text(
                        egui::pos2(rect.left() + t.space_md, rect.bottom() - t.space_sm),
                        egui::Align2::LEFT_BOTTOM,
                        &hit.label,
                        egui::FontId::proportional(t.font_body),
                        p.ink,
                    );
                    if response.clicked() {
                        jump = Some((hit.page, hit.focus_id));
                    }
                    ui.add_space(t.space_xs);
                }
            });

        if let Some((page, focus_id)) = jump {
            app.settings_page = page;
            app.settings_search.clear();
            app.focus_target = focus_id;
        }
    });
}

fn page_label(page: Page) -> &'static str {
    PAGES.iter().find(|(p, _)| *p == page).map(|(_, l)| *l).unwrap_or("")
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
