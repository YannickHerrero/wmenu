use eframe::egui::{self, Key, Ui};

use crate::index::AppEntry;
use crate::matcher::Engine;
use crate::mru::Mru;

const MAX_VISIBLE_RESULTS: usize = 50;

pub enum Action {
    None,
    Launch(usize),
    Hide,
}

pub fn show(
    ui: &mut Ui,
    query: &mut String,
    selected: &mut usize,
    entries: &[AppEntry],
    matcher: &mut Engine,
    mru: &Mru,
    request_focus: bool,
) -> Action {
    let ranked = matcher.search(query, entries, mru);
    if ranked.is_empty() {
        *selected = 0;
    } else if *selected >= ranked.len() {
        *selected = ranked.len() - 1;
    }

    let mut action = Action::None;

    let response = ui.add(
        egui::TextEdit::singleline(query)
            .desired_width(f32::INFINITY)
            .hint_text("Search…"),
    );
    if request_focus {
        response.request_focus();
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

    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            for (display_idx, &entry_idx) in ranked.iter().enumerate().take(MAX_VISIBLE_RESULTS) {
                let entry = &entries[entry_idx];
                let is_sel = display_idx == *selected;
                let row = ui.add(egui::SelectableLabel::new(is_sel, &entry.name));
                if row.clicked() {
                    action = Action::Launch(entry_idx);
                }
                if is_sel && request_focus {
                    row.scroll_to_me(None);
                }
            }
        });

    action
}
