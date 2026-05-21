use eframe::egui::{self, Frame, Key, Margin, Sense, Ui};

use crate::omakase::{Page, SystemAction};
use crate::ui::theme::Palette;

pub enum Action {
    None,
    Back,
    Hide,
    EnterSystem,
    EnterHelp,
    ToggleAmphetamine,
    SelectSystem(SystemAction),
    ConfirmSystem(SystemAction),
}

pub fn show(
    ui: &mut Ui,
    palette: &Palette,
    page: Page,
    query: &mut String,
    selected: &mut usize,
    amphetamine_on: bool,
    request_focus: bool,
) -> Action {
    match page {
        Page::Top => show_top(ui, palette, query, selected, amphetamine_on, request_focus),
        Page::System => show_system(ui, palette, query, selected, request_focus),
        Page::Confirm(action) => show_confirm(ui, palette, action, request_focus),
        Page::Help => show_help(ui, palette),
    }
}

fn show_top(
    ui: &mut Ui,
    palette: &Palette,
    query: &mut String,
    selected: &mut usize,
    amphetamine_on: bool,
    request_focus: bool,
) -> Action {
    let amph_label = if amphetamine_on {
        "Amphetamine: ON"
    } else {
        "Amphetamine: OFF"
    };
    let items: Vec<(&str, TopAction)> = vec![
        ("System", TopAction::EnterSystem),
        (amph_label, TopAction::ToggleAmphetamine),
        ("Help", TopAction::EnterHelp),
    ];

    match menu_pick(
        ui,
        palette,
        "Omakase",
        query,
        selected,
        &items,
        request_focus,
    ) {
        MenuResult::Hide => Action::Hide,
        MenuResult::Pick(TopAction::EnterSystem) => Action::EnterSystem,
        MenuResult::Pick(TopAction::ToggleAmphetamine) => Action::ToggleAmphetamine,
        MenuResult::Pick(TopAction::EnterHelp) => Action::EnterHelp,
        MenuResult::None => Action::None,
    }
}

fn show_system(
    ui: &mut Ui,
    palette: &Palette,
    query: &mut String,
    selected: &mut usize,
    request_focus: bool,
) -> Action {
    let items: Vec<(&str, SystemAction)> = vec![
        (SystemAction::Shutdown.label(), SystemAction::Shutdown),
        (SystemAction::Restart.label(), SystemAction::Restart),
        (SystemAction::Hibernate.label(), SystemAction::Hibernate),
    ];

    match menu_pick(
        ui,
        palette,
        "System",
        query,
        selected,
        &items,
        request_focus,
    ) {
        MenuResult::Hide => Action::Back,
        MenuResult::Pick(action) => Action::SelectSystem(action),
        MenuResult::None => Action::None,
    }
}

fn show_confirm(
    ui: &mut Ui,
    palette: &Palette,
    action: SystemAction,
    request_focus: bool,
) -> Action {
    let mut result = Action::None;

    ui.input(|i| {
        if i.key_pressed(Key::Escape) {
            result = Action::Back;
        }
    });

    Frame::default()
        .fill(palette.paper)
        .inner_margin(Margin::symmetric(16, 20))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.colored_label(palette.ink_soft, "Confirm");
                ui.add_space(8.0);
                ui.colored_label(palette.ink, format!("{} now?", action.label()));
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2.0 - 80.0);
                    let cancel = ui.button("Cancel");
                    if request_focus {
                        cancel.request_focus();
                    }
                    if cancel.clicked() {
                        result = Action::Back;
                    }
                    if ui.button("Confirm").clicked() {
                        result = Action::ConfirmSystem(action);
                    }
                });
            });
        });

    result
}

fn show_help(ui: &mut Ui, palette: &Palette) -> Action {
    let mut action = Action::None;

    ui.input(|i| {
        if i.key_pressed(Key::Escape) {
            action = Action::Back;
        }
    });

    Frame::default()
        .fill(palette.paper)
        .inner_margin(Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.colored_label(palette.ink, "wmenu — keybindings");
            ui.colored_label(palette.ink_faint, format!("v{}", env!("CARGO_PKG_VERSION")));
            ui.add_space(10.0);

            for (k, v) in KEYBINDINGS {
                ui.horizontal(|ui| {
                    ui.colored_label(palette.accent, *k);
                    ui.add_space(8.0);
                    ui.colored_label(palette.ink, *v);
                });
            }

            ui.add_space(10.0);
            ui.colored_label(palette.ink_faint, "Esc to return");
        });

    action
}

const KEYBINDINGS: &[(&str, &str)] = &[
    ("Alt+Space        ", "Open launcher"),
    ("Alt+Super+Space  ", "Open omakase menu"),
    ("Ctrl+,           ", "Open settings"),
    ("↑ / ↓            ", "Navigate"),
    ("Enter            ", "Select / launch"),
    ("Esc              ", "Back / dismiss"),
];

#[derive(Clone, Copy)]
enum TopAction {
    EnterSystem,
    ToggleAmphetamine,
    EnterHelp,
}

enum MenuResult<A: Copy> {
    None,
    Pick(A),
    Hide,
}

fn menu_pick<A: Copy>(
    ui: &mut Ui,
    palette: &Palette,
    title: &str,
    query: &mut String,
    selected: &mut usize,
    items: &[(&str, A)],
    request_focus: bool,
) -> MenuResult<A> {
    let prev_selected = *selected;

    let filtered: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, (name, _))| matches_filter(name, query))
        .map(|(i, _)| i)
        .collect();
    if filtered.is_empty() {
        *selected = 0;
    } else if *selected >= filtered.len() {
        *selected = filtered.len() - 1;
    }

    let mut result = MenuResult::None;

    let search_frame = egui::Frame::default()
        .fill(palette.muted)
        .inner_margin(Margin::symmetric(8, 6));
    let search_response = search_frame
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(palette.ink_faint, title);
                ui.add(
                    egui::TextEdit::singleline(query)
                        .desired_width(f32::INFINITY)
                        .text_color(palette.ink)
                        .hint_text("Filter…")
                        .frame(Frame::NONE),
                )
            })
            .inner
        })
        .inner;
    if request_focus {
        search_response.request_focus();
    }
    if search_response.changed() {
        *selected = 0;
    }

    let visible = filtered.len();
    ui.input(|i| {
        if i.key_pressed(Key::Escape) {
            result = MenuResult::Hide;
        } else if i.key_pressed(Key::Enter) && !filtered.is_empty() {
            let (_, action) = items[filtered[*selected]];
            result = MenuResult::Pick(action);
        } else if i.key_pressed(Key::ArrowDown) && visible > 0 {
            *selected = (*selected + 1).min(visible - 1);
        } else if i.key_pressed(Key::ArrowUp) {
            *selected = selected.saturating_sub(1);
        }
    });

    let scroll = request_focus || *selected != prev_selected;

    ui.add(egui::Separator::default().spacing(0.0));

    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            for (display_idx, &item_idx) in filtered.iter().enumerate() {
                let (name, action) = items[item_idx];
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
                        ui.colored_label(fg, name);
                    });
                let clickable = ui.interact(
                    row.response.rect,
                    row.response.id.with(item_idx),
                    Sense::click(),
                );
                if clickable.clicked() {
                    result = MenuResult::Pick(action);
                }
                if is_sel && scroll {
                    clickable.scroll_to_me(None);
                }
            }
        });

    result
}

fn matches_filter(name: &str, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    name.to_lowercase().contains(&q)
}
