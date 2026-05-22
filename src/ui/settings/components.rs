//! Reusable layout primitives for the settings pages.
//!
//! These helpers exist so individual pages stop sprinkling bare
//! `add_space(8.0)`, `ui.indent`, ad-hoc red labels, etc. Every page should
//! compose itself from `page_header`, `section`, `field_row`, and the small
//! status/error widgets below.

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Frame, Margin, Response, RichText, Sense,
    Stroke, TextEdit, Ui, Vec2,
};

use crate::config::Theme;
use crate::hotkey_spec::HotkeySpec;
use crate::index::SharedIndex;
use crate::matcher::Engine;
use crate::mru::Mru;
use crate::ui::theme;

/// What kind of pill / inline error to render.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Neutral,
    Success,
    #[allow(dead_code)] // reserved for upcoming warning surfaces
    Warning,
    Error,
}

/// Wraps the body of a settings page in a consistent outer gutter so the
/// content doesn't stick to the panel edges. Every page should call this
/// once at the top.
pub fn page_frame(ui: &mut Ui, theme: Theme, body: impl FnOnce(&mut Ui)) {
    let t = theme::tokens();
    let p = theme::palette(theme);
    Frame::new()
        .fill(p.paper)
        .inner_margin(Margin {
            left: t.space_xl as i8,
            right: t.space_xl as i8,
            top: 0,
            bottom: t.space_lg as i8,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            body(ui);
        });
}

/// Large page heading with optional subtitle and a hairline divider beneath.
pub fn page_header(ui: &mut Ui, theme: Theme, title: &str, subtitle: Option<&str>) {
    let t = theme::tokens();
    let p = theme::palette(theme);
    ui.add_space(t.space_md);
    ui.label(
        RichText::new(title)
            .size(t.font_page_title)
            .color(p.ink)
            .strong(),
    );
    if let Some(sub) = subtitle {
        ui.add_space(t.space_xs);
        ui.label(RichText::new(sub).size(t.font_body).color(p.ink_soft));
    }
    ui.add_space(t.space_sm);
    hairline(ui, p.ink_faint);
    ui.add_space(t.space_md);
}

/// Section with a small caps-style title and a body indented under it.
pub fn section(ui: &mut Ui, theme: Theme, title: &str, body: impl FnOnce(&mut Ui)) {
    let t = theme::tokens();
    let p = theme::palette(theme);
    ui.add_space(t.space_md);
    ui.label(
        RichText::new(title.to_uppercase())
            .size(t.font_section_title)
            .color(p.ink_soft)
            .strong(),
    );
    ui.add_space(t.space_sm);
    body(ui);
}

/// A label-on-the-left, control-on-the-right row. The label is rendered with
/// a fixed width so multiple field rows line up vertically.
pub fn field_row(ui: &mut Ui, theme: Theme, label: &str, body: impl FnOnce(&mut Ui)) {
    let t = theme::tokens();
    let p = theme::palette(theme);
    ui.horizontal(|ui| {
        ui.set_min_height(t.space_lg);
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(t.field_label_width, t.space_lg), Sense::hover());
        ui.painter().text(
            rect.left_center(),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(t.font_body),
            p.ink,
        );
        body(ui);
    });
    ui.add_space(t.space_xs);
}

/// Variant of [`field_row`] for stacked layouts where the control sits below
/// its label (long multilines, list editors, etc.).
pub fn stacked_field(ui: &mut Ui, theme: Theme, label: &str, body: impl FnOnce(&mut Ui)) {
    let t = theme::tokens();
    let p = theme::palette(theme);
    ui.label(RichText::new(label).size(t.font_body).color(p.ink));
    ui.add_space(t.space_xs);
    body(ui);
    ui.add_space(t.space_xs);
}

/// Small inline error/warning row meant to sit directly below the field that
/// caused it. Renders an icon glyph and the message in the semantic colour.
pub fn inline_error(ui: &mut Ui, theme: Theme, message: &str) -> Response {
    inline_status(ui, theme, Kind::Error, "⚠", message)
}

/// Generic inline status row (info / warning / error).
pub fn inline_status(
    ui: &mut Ui,
    theme: Theme,
    kind: Kind,
    glyph: &str,
    message: &str,
) -> Response {
    let t = theme::tokens();
    let (fg, _bg) = status_colors(theme, kind);
    ui.horizontal(|ui| {
        ui.add_space(t.space_xs);
        ui.label(
            RichText::new(glyph)
                .size(t.font_body)
                .color(fg)
                .strong(),
        );
        ui.label(RichText::new(message).size(t.font_body).color(fg));
    })
    .response
}

/// Small rounded badge ("pill") — used for auto-save status and the like.
pub fn pill(ui: &mut Ui, theme: Theme, kind: Kind, text: &str) -> Response {
    let t = theme::tokens();
    let (fg, bg) = status_colors(theme, kind);
    Frame::new()
        .fill(bg)
        .stroke(Stroke::new(1.0, fg))
        .corner_radius(CornerRadius::same(t.radius_md as u8))
        .inner_margin(Margin {
            left: t.space_sm as i8,
            right: t.space_sm as i8,
            top: t.space_xs as i8,
            bottom: t.space_xs as i8,
        })
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(t.font_body).color(fg));
        })
        .response
}

/// Thin horizontal divider used to underline section/page headers.
pub fn hairline(ui: &mut Ui, color: Color32) {
    let rect = ui.available_rect_before_wrap();
    let y = rect.top();
    ui.painter().line_segment(
        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
        Stroke::new(1.0, color),
    );
    ui.add_space(1.0);
}

/// Soft container used to group a single record (e.g. one keybinding) inside
/// a list. Subtle border + light tint, rounded corners.
pub fn card<R>(ui: &mut Ui, theme: Theme, body: impl FnOnce(&mut Ui) -> R) -> R {
    let t = theme::tokens();
    let p = theme::palette(theme);
    Frame::new()
        .fill(p.muted)
        .stroke(Stroke::new(1.0, p.ink_faint))
        .corner_radius(CornerRadius::same(t.radius_md as u8))
        .inner_margin(Margin::same(t.space_md as i8))
        .show(ui, body)
        .inner
}

fn status_colors(theme: Theme, kind: Kind) -> (Color32, Color32) {
    let p = theme::palette(theme);
    let fg = match kind {
        Kind::Neutral => p.ink_soft,
        Kind::Success => p.success,
        Kind::Warning => p.warning,
        Kind::Error => p.error,
    };
    let bg = tint(p.paper, fg, 0.12);
    (fg, bg)
}

/// Blend `base` toward `accent` by `amount` (0.0..=1.0). Used to derive tinted
/// pill backgrounds from the foreground colour.
fn tint(base: Color32, accent: Color32, amount: f32) -> Color32 {
    let a = amount.clamp(0.0, 1.0);
    let inv = 1.0 - a;
    let mix = |b: u8, t: u8| (b as f32 * inv + t as f32 * a) as u8;
    Color32::from_rgb(
        mix(base.r(), accent.r()),
        mix(base.g(), accent.g()),
        mix(base.b(), accent.b()),
    )
}

/// Result of [`hotkey_input`] for a single frame.
pub struct HotkeyInputResult {
    /// `Some(spec)` when the current buffer parses cleanly; otherwise `None`.
    pub spec: Option<HotkeySpec>,
    /// Response of the text edit so the caller can request focus etc. The
    /// caller can also read `response.changed()` if it cares about frame-level
    /// edit detection.
    pub response: Response,
}

/// Renders a hotkey text input with a live parse preview underneath.
///
/// - Empty input: preview is suppressed (the cheatsheet still teaches format).
/// - Valid parse: small green tick + the expanded "Ctrl + Shift + Enter" form.
/// - Invalid parse: small red X + the [`HotkeySpec::parse`] error message.
///
/// `extra_error` is rendered after the parse preview and is intended for
/// errors that come from the registration layer (e.g. "duplicate of binding
/// #N", "register failed"), which the parser can't detect on its own.
pub fn hotkey_input(
    ui: &mut Ui,
    theme: Theme,
    buf: &mut String,
    width: f32,
    extra_error: Option<&str>,
) -> HotkeyInputResult {
    let t = theme::tokens();
    let p = theme::palette(theme);

    let response = ui.add_sized([width, 28.0], TextEdit::singleline(buf));

    let trimmed = buf.trim();
    let parsed = (!trimmed.is_empty()).then(|| HotkeySpec::parse(trimmed));

    ui.add_space(t.space_xs);
    match parsed.as_ref() {
        Some(Ok(spec)) => {
            ui.label(
                RichText::new(format!("✓ {}", spec.to_human()))
                    .small()
                    .color(p.success),
            );
        }
        Some(Err(err)) => {
            ui.label(
                RichText::new(format!("✗ {err}"))
                    .small()
                    .color(p.error),
            );
        }
        None => {
            // Reserve the line height so the layout doesn't jump when the
            // user starts typing.
            ui.label(RichText::new(" ").small());
        }
    }

    if let Some(err) = extra_error {
        ui.label(
            RichText::new(format!("⚠ {err}"))
                .small()
                .color(p.error),
        );
    }

    HotkeyInputResult {
        spec: parsed.and_then(|r| r.ok()),
        response,
    }
}

/// One-line legend for the hotkey input format. Render once near the top of a
/// section that contains hotkey fields so the user can learn the AHK
/// shorthand without having to memorise it.
pub fn hotkey_cheatsheet(ui: &mut Ui, theme: Theme) {
    let p = theme::palette(theme);
    ui.label(
        RichText::new(
            "Modifiers: ^ Ctrl  ·  + Shift  ·  ! Alt  ·  # Super     \
             Keys: A–Z, 0–9, F1–F24, Enter, Tab, Space, Esc, Up/Down/Left/Right, …",
        )
        .small()
        .color(p.ink_soft),
    );
}

/// Whether the picker should store the raw `.lnk` path it picked or the
/// underlying `.exe` target. `Launch` actions want `Lnk` (cmd start handles
/// shortcuts transparently), `FocusOrLaunch` wants `ResolvedExe` so its
/// window-process matching has a real .exe to compare against.
#[allow(dead_code)] // wired in following commits
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PickerMode {
    Lnk,
    ResolvedExe,
}

/// External state the picker needs each frame. Borrows rather than ownership
/// so the caller can keep mutating the same `App` fields elsewhere in the
/// frame without lifetimes getting in the way.
#[allow(dead_code)] // fields consumed by following commits
pub struct AppPickerCtx<'a> {
    pub index: &'a SharedIndex,
    pub matcher: &'a mut Engine,
    pub mru: &'a Mru,
    /// Unique per-row id so multiple pickers on the same page (one per
    /// binding) keep separate open/selected state in egui memory.
    pub state_id: egui::Id,
}

/// Per-picker UI state stashed in `egui::Memory::data`.
#[allow(dead_code)] // `selected` is consumed by the keyboard nav commit
#[derive(Clone, Default)]
struct PickerState {
    /// Whether the dropdown is currently open. Toggled by focus and Esc.
    open: bool,
    /// Index into the matched results that's currently highlighted.
    selected: usize,
}

/// Maximum number of matching apps shown in the dropdown.
const PICKER_MAX_RESULTS: usize = 10;

/// Text input with an inline autocomplete dropdown of indexed Start-Menu
/// apps. The buffer stays editable for arbitrary manual paths; the dropdown
/// only opens when the field has focus and there's text to match against.
#[allow(dead_code)] // wired in the bindings page in a later commit
pub fn app_picker(
    ui: &mut Ui,
    theme: Theme,
    buf: &mut String,
    ctx: &mut AppPickerCtx<'_>,
    _mode: PickerMode,
) -> Response {
    let id = ctx.state_id;
    let t = theme::tokens();
    let p = theme::palette(theme);

    let mut state: PickerState = ui
        .ctx()
        .memory(|m| m.data.get_temp::<PickerState>(id).unwrap_or_default());

    let response = ui.add(
        TextEdit::singleline(buf)
            .desired_width(f32::INFINITY)
            .id(id.with("input")),
    );

    state.open = response.has_focus();

    if state.open {
        let snapshot = ctx.index.load();
        let ranked = ctx.matcher.search(buf, &snapshot.entries, ctx.mru);
        let visible = ranked.len().min(PICKER_MAX_RESULTS);
        if visible > 0 {
            if state.selected >= visible {
                state.selected = visible - 1;
            }

            let input_rect = response.rect;
            let area_id = id.with("dropdown");
            egui::Area::new(area_id)
                .order(egui::Order::Foreground)
                .fixed_pos(egui::pos2(input_rect.left(), input_rect.bottom() + t.space_xs))
                .show(ui.ctx(), |ui| {
                    Frame::new()
                        .fill(p.paper)
                        .stroke(Stroke::new(1.0, p.ink_faint))
                        .corner_radius(CornerRadius::same(t.radius_sm as u8))
                        .inner_margin(Margin::same(t.space_xs as i8))
                        .show(ui, |ui| {
                            ui.set_width(input_rect.width());
                            for (display_idx, &entry_idx) in
                                ranked.iter().enumerate().take(PICKER_MAX_RESULTS)
                            {
                                let entry = &snapshot.entries[entry_idx];
                                let is_sel = display_idx == state.selected;
                                let bg = if is_sel { p.accent } else { p.paper };
                                let fg = if is_sel { p.paper } else { p.ink };
                                let row = Frame::new()
                                    .fill(bg)
                                    .inner_margin(Margin::symmetric(
                                        t.space_sm as i8,
                                        t.space_xs as i8,
                                    ))
                                    .corner_radius(CornerRadius::same(2))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.label(
                                            RichText::new(&entry.name)
                                                .size(t.font_body)
                                                .color(fg),
                                        );
                                    });
                                let click = ui.interact(
                                    row.response.rect,
                                    area_id.with(entry_idx),
                                    Sense::click(),
                                );
                                if click.hovered() {
                                    state.selected = display_idx;
                                }
                                if click.clicked() {
                                    *buf = entry.path.to_string_lossy().into_owned();
                                }
                            }
                        });
                });
        }
    }

    ui.ctx().memory_mut(|m| m.data.insert_temp(id, state));
    response
}

/// Build the egui::Id a focusable settings field should use. Search results
/// can request focus on the matching widget by storing the same string in
/// `App.focus_target`.
pub fn focus_id(name: &'static str) -> egui::Id {
    egui::Id::new(("settings_focus", name))
}

/// If `App.focus_target == Some(name)`, calls `request_focus()` on `resp` and
/// clears the target so the hand-off only fires once.
pub fn consume_focus_target(
    response: &egui::Response,
    target: &mut Option<&'static str>,
    name: &'static str,
) {
    if *target == Some(name) {
        response.request_focus();
        *target = None;
    }
}

