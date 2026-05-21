use eframe::egui::{self, Frame, Key, Margin, Sense, Ui};

use crate::index::AppEntry;
use crate::matcher::Engine;
use crate::mru::Mru;
use crate::ui::theme::Palette;

const MAX_VISIBLE_RESULTS: usize = 50;

pub enum Action {
    None,
    Launch(usize),
    Hide,
}

pub fn show(
    ui: &mut Ui,
    palette: &Palette,
    query: &mut String,
    selected: &mut usize,
    entries: &[AppEntry],
    matcher: &mut Engine,
    mru: &Mru,
    request_focus: bool,
) -> Action {
    let prev_selected = *selected;

    let ranked = matcher.search(query, entries, mru);
    if ranked.is_empty() {
        *selected = 0;
    } else if *selected >= ranked.len() {
        *selected = ranked.len() - 1;
    }

    let mut action = Action::None;

    let search_frame = egui::Frame::default()
        .fill(palette.muted)
        .inner_margin(Margin::symmetric(8, 6));
    let search_response = search_frame
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(query)
                    .desired_width(f32::INFINITY)
                    .text_color(palette.ink)
                    .hint_text("Search…")
                    .frame(Frame::NONE),
            )
        })
        .inner;
    if request_focus {
        search_response.request_focus();
    }

    let visible = ranked.len().min(MAX_VISIBLE_RESULTS);
    ui.input(|i| {
        if i.key_pressed(Key::Escape) {
            action = Action::Hide;
        } else if i.key_pressed(Key::Enter) && !ranked.is_empty() {
            action = Action::Launch(ranked[*selected]);
        } else if i.key_pressed(Key::ArrowDown) && visible > 0 {
            *selected = (*selected + 1).min(visible - 1);
        } else if i.key_pressed(Key::ArrowUp) {
            *selected = selected.saturating_sub(1);
        }
    });

    let scroll_to_selected = request_focus || *selected != prev_selected;

    ui.add(egui::Separator::default().spacing(0.0));

    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            for (display_idx, &entry_idx) in ranked.iter().enumerate().take(MAX_VISIBLE_RESULTS) {
                let entry = &entries[entry_idx];
                let is_sel = display_idx == *selected;
                let bg = if is_sel {
                    palette.accent
                } else {
                    palette.paper
                };
                let fg = if is_sel { palette.paper } else { palette.ink };
                let row = egui::Frame::default()
                    .fill(bg)
                    .inner_margin(Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.colored_label(fg, &entry.name);
                    });
                let clickable = ui.interact(
                    row.response.rect,
                    row.response.id.with(entry_idx),
                    Sense::click(),
                );
                if clickable.clicked() {
                    action = Action::Launch(entry_idx);
                }
                if is_sel && scroll_to_selected {
                    clickable.scroll_to_me(None);
                }
            }
        });

    action
}
