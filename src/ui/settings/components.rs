//! Reusable layout primitives for the settings pages.
//!
//! These helpers exist so individual pages stop sprinkling bare
//! `add_space(8.0)`, `ui.indent`, ad-hoc red labels, etc. Every page should
//! compose itself from `page_header`, `section`, `field_row`, and the small
//! status/error widgets below.

#![allow(dead_code)] // wired in over the next commits

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, FontSelection, Frame, Layout, Margin,
    Response, RichText, Sense, Stroke, StrokeKind, Ui, Vec2,
};

use crate::config::Theme;
use crate::ui::theme::{self, Tokens};

/// What kind of pill / inline error to render.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Neutral,
    Success,
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

/// Right-aligned trailing-control helper for a [`field_row`] body that should
/// hug the right side of the page.
pub fn trailing(ui: &mut Ui, body: impl FnOnce(&mut Ui)) {
    ui.with_layout(Layout::right_to_left(Align::Center), body);
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

/// Helper to draw a focus ring around the previously-allocated rect.
pub fn focus_ring(ui: &Ui, rect: egui::Rect, accent: Color32) {
    let t = theme::tokens();
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(t.radius_sm as u8),
        Stroke::new(2.0, accent),
        StrokeKind::Outside,
    );
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

/// Tokens accessor exposed here to avoid every page reaching into
/// `crate::ui::theme` directly.
pub fn t() -> Tokens {
    theme::tokens()
}

/// Helper: convert a plain string into a body-sized rich text in the page's
/// default text colour. Lets pages write `body("hello")` instead of repeating
/// the size/colour boilerplate everywhere.
pub fn body(theme: Theme, text: impl Into<String>) -> RichText {
    let t = theme::tokens();
    RichText::new(text.into())
        .size(t.font_body)
        .color(theme::palette(theme).ink)
}

/// FontSelection used by inputs to ensure the font_body size is applied.
pub fn input_font() -> FontSelection {
    FontSelection::FontId(FontId::proportional(theme::tokens().font_body))
}
