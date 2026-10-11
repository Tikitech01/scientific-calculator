//! Small hand-drawn widgets shared by every mode.

use crate::theme::{self, mix, Pal};
use eframe::egui::{
    self, text::CCursor, text::CCursorRange, Align, Align2, Color32, CornerRadius, CursorIcon, FontId, Frame, Id,
    Margin, Response, RichText, Sense, Stroke, StrokeKind, TextEdit, Ui, Vec2,
};

#[derive(Clone, Copy, PartialEq)]
pub enum KeyKind {
    Digit,
    Func,
    Op,
    Accent,
}

/// A keypad key that allocates its own space.
pub fn key(ui: &mut Ui, pal: &Pal, label: &str, kind: KeyKind, size: Vec2, active: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    paint_key(ui, pal, rect, &resp, label, kind, active);
    resp
}

/// A keypad key at an explicit position.
pub fn key_at(ui: &mut Ui, pal: &Pal, rect: egui::Rect, id: impl std::hash::Hash, label: &str, kind: KeyKind, active: bool) -> Response {
    let resp = ui.interact(rect, ui.id().with(id), Sense::click());
    paint_key(ui, pal, rect, &resp, label, kind, active);
    resp
}

fn paint_key(ui: &Ui, pal: &Pal, rect: egui::Rect, resp: &Response, label: &str, kind: KeyKind, active: bool) {
    let hover = ui.ctx().animate_bool_with_time(resp.id.with("hover"), resp.hovered(), 0.10);
    let down = resp.is_pointer_button_down_on();
    let (base, over, pressed, text) = match kind {
        KeyKind::Digit => (pal.surface, mix(pal.surface, pal.key, 0.7), pal.key_hover, pal.text),
        KeyKind::Func => (pal.bg, pal.key, pal.key_hover, pal.dim),
        KeyKind::Op => (pal.key, pal.key_hover, pal.key_down, pal.text),
        KeyKind::Accent => (pal.accent, pal.accent_hover, mix(pal.accent_hover, pal.text, 0.12), pal.on_accent),
    };
    let mut fill = if down { pressed } else { mix(base, over, hover) };
    let mut text_color = if kind == KeyKind::Func { mix(text, pal.text, hover) } else { text };
    if active {
        fill = mix(pal.bg, pal.accent, 0.16);
        text_color = pal.accent;
    }
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(9), fill);
    if kind == KeyKind::Digit && !pal.dark {
        p.rect_stroke(rect, CornerRadius::same(9), Stroke::new(1.0_f32, pal.line), StrokeKind::Inside);
    }
    let font = match kind {
        KeyKind::Digit => theme::regular(18.0),
        KeyKind::Func => theme::regular(14.0),
        KeyKind::Op | KeyKind::Accent => theme::regular(19.0),
    };
    let font = if label.chars().count() > 3 && kind != KeyKind::Func { theme::regular(14.0) } else { font };
    p.text(rect.center(), Align2::CENTER_CENTER, label, font, text_color);
}

/// Text-only tab. Returns true when clicked.
pub fn tab(ui: &mut Ui, pal: &Pal, label: &str, selected: bool) -> bool {
    let font = if selected { theme::medium(13.5) } else { theme::regular(13.5) };
    let galley = ui.painter().layout_no_wrap(label.to_string(), font, Color32::PLACEHOLDER);
    let size = galley.size() + egui::vec2(20.0, 12.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hover = ui.ctx().animate_bool_with_time(resp.id.with("hover"), resp.hovered(), 0.10);
    let color = if selected { pal.text } else { mix(pal.dim, pal.text, hover) };
    if selected {
        ui.painter().rect_filled(rect, CornerRadius::same(7), pal.key);
    }
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, color);
    resp.on_hover_cursor(CursorIcon::PointingHand).clicked()
}

/// Pill-style segmented control. Returns true when the selection changed.
pub fn segmented(ui: &mut Ui, pal: &Pal, sel: &mut usize, labels: &[&str]) -> bool {
    let mut changed = false;
    Frame::new().fill(pal.key).corner_radius(CornerRadius::same(8)).inner_margin(Margin::same(2)).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            // Keep reading order even inside a right-to-left parent layout.
            let rtl = ui.layout().prefer_right_to_left();
            let order: Vec<usize> = if rtl { (0..labels.len()).rev().collect() } else { (0..labels.len()).collect() };
            for i in order {
                let label = &labels[i];
                let selected = *sel == i;
                let font = if selected { theme::medium(12.5) } else { theme::regular(12.5) };
                let galley = ui.painter().layout_no_wrap(label.to_string(), font, Color32::PLACEHOLDER);
                let size = galley.size() + egui::vec2(18.0, 9.0);
                let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
                let hover = ui.ctx().animate_bool_with_time(resp.id.with("hover"), resp.hovered(), 0.10);
                if selected {
                    ui.painter().rect_filled(rect, CornerRadius::same(6), pal.surface);
                }
                let color = if selected { pal.text } else { mix(pal.dim, pal.text, hover) };
                ui.painter().galley(rect.center() - galley.size() / 2.0, galley, color);
                if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() && !selected {
                    *sel = i;
                    changed = true;
                }
            }
        });
    });
    changed
}

/// Quiet text button.
pub fn link(ui: &mut Ui, pal: &Pal, label: &str) -> bool {
    let galley = ui.painter().layout_no_wrap(label.to_string(), theme::regular(13.0), Color32::PLACEHOLDER);
    let size = galley.size() + egui::vec2(16.0, 10.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hover = ui.ctx().animate_bool_with_time(resp.id.with("hover"), resp.hovered(), 0.10);
    ui.painter().rect_filled(rect, CornerRadius::same(7), mix(pal.key, pal.key_hover, hover));
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, pal.text);
    resp.on_hover_cursor(CursorIcon::PointingHand).clicked()
}

/// Small square icon-like button with a text glyph.
pub fn glyph(ui: &mut Ui, pal: &Pal, label: &str, size: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let hover = ui.ctx().animate_bool_with_time(resp.id.with("hover"), resp.hovered(), 0.10);
    if hover > 0.0 {
        ui.painter().rect_filled(rect, CornerRadius::same(6), mix(pal.bg, pal.key_hover, hover));
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        theme::regular(size * 0.55),
        mix(pal.dim, pal.text, hover),
    );
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// Framed single-line text field.
pub fn field(ui: &mut Ui, pal: &Pal, text: &mut String, hint: &str, width: f32) -> Response {
    field_with(ui, pal, text, hint, width, Align::LEFT, None)
}

pub fn field_with(
    ui: &mut Ui,
    pal: &Pal,
    text: &mut String,
    hint: &str,
    width: f32,
    align: Align,
    id: Option<Id>,
) -> Response {
    Frame::new()
        .fill(pal.surface)
        .stroke(Stroke::new(1.0_f32, pal.line))
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::symmetric(9, 6))
        .show(ui, |ui| {
            let mut te = TextEdit::singleline(text)
                .frame(false)
                .hint_text(RichText::new(hint).color(pal.faint))
                .desired_width(width)
                .horizontal_align(align)
                .text_color(pal.text)
                .font(theme::regular(14.0));
            if let Some(id) = id {
                te = te.id(id);
            }
            let resp = ui.add(te);
            if resp.changed() || resp.lost_focus() {
                prettify_field(ui.ctx(), resp.id, text, resp.lost_focus());
            }
            resp
        })
        .inner
}

/// Rewrite typed powers (x^2) as raised digits, keeping the caret where the user left it.
pub fn prettify_field(ctx: &egui::Context, id: Id, text: &mut String, finalize: bool) {
    let pretty = crate::fmt::prettify(text, finalize);
    if pretty == *text {
        return;
    }
    let delta = pretty.chars().count() as isize - text.chars().count() as isize;
    *text = pretty;
    if let Some(mut state) = TextEdit::load_state(ctx, id) {
        if let Some(r) = state.cursor.char_range() {
            let pos = (r.primary.index as isize + delta).max(0) as usize;
            state.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(pos))));
            state.store(ctx, id);
        }
    }
}

/// Uppercase-ish section caption.
pub fn caption(ui: &mut Ui, pal: &Pal, text: &str) {
    ui.label(RichText::new(text).font(theme::medium(11.0)).color(pal.dim));
}

pub fn dim(ui: &mut Ui, pal: &Pal, text: impl Into<String>) {
    ui.label(RichText::new(text.into()).color(pal.dim).size(13.0));
}

pub fn error(ui: &mut Ui, pal: &Pal, text: impl Into<String>) {
    ui.label(RichText::new(text.into()).color(pal.danger).size(12.5));
}

/// A label / value row used in result lists.
pub fn row(ui: &mut Ui, pal: &Pal, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.set_min_height(22.0);
        ui.label(RichText::new(label).color(pal.dim).size(13.5));
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            ui.add(egui::Label::new(RichText::new(value).size(14.5)).selectable(true));
        });
    });
}

/// A card container.
pub fn card<R>(ui: &mut Ui, pal: &Pal, add: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new()
        .fill(pal.surface)
        .stroke(Stroke::new(1.0_f32, pal.line))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(14))
        .show(ui, add)
        .inner
}

fn char_to_byte(s: &str, ci: usize) -> usize {
    s.char_indices().nth(ci).map(|x| x.0).unwrap_or(s.len())
}

fn selection(ctx: &egui::Context, id: Id, s: &str) -> (egui::text_edit::TextEditState, usize, usize) {
    let state = TextEdit::load_state(ctx, id).unwrap_or_default();
    let n = s.chars().count();
    let (a, b) = match state.cursor.char_range() {
        Some(r) => {
            let (p, q) = (r.primary.index.min(n), r.secondary.index.min(n));
            (p.min(q), p.max(q))
        }
        None => (n, n),
    };
    (state, a, b)
}

fn place_cursor(ctx: &egui::Context, id: Id, mut state: egui::text_edit::TextEditState, pos: usize) {
    state.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(pos))));
    state.store(ctx, id);
    ctx.memory_mut(|m| m.request_focus(id));
}

/// Insert text at the cursor of the TextEdit with the given id (replacing any selection).
pub fn insert_at_cursor(ctx: &egui::Context, id: Id, s: &mut String, text: &str) {
    let (state, a, b) = selection(ctx, id, s);
    s.replace_range(char_to_byte(s, a)..char_to_byte(s, b), text);
    place_cursor(ctx, id, state, a + text.chars().count());
}

pub fn backspace_at_cursor(ctx: &egui::Context, id: Id, s: &mut String) {
    let (state, a, b) = selection(ctx, id, s);
    let (a, b) = if a == b { (a.saturating_sub(1), b) } else { (a, b) };
    s.replace_range(char_to_byte(s, a)..char_to_byte(s, b), "");
    place_cursor(ctx, id, state, a);
}

pub fn cursor_to_end(ctx: &egui::Context, id: Id, s: &str) {
    let state = TextEdit::load_state(ctx, id).unwrap_or_default();
    place_cursor(ctx, id, state, s.chars().count());
}

pub fn text_width(ui: &Ui, text: &str, font: FontId) -> f32 {
    ui.painter().layout_no_wrap(text.to_string(), font, Color32::PLACEHOLDER).size().x
}

/// Round half-filled button that stands for light / dark appearance.
pub fn appearance_button(ui: &mut Ui, pal: &Pal, size: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let hover = ui.ctx().animate_bool_with_time(resp.id.with("hover"), resp.hovered(), 0.10);
    if hover > 0.0 {
        ui.painter().rect_filled(rect, CornerRadius::same(6), mix(pal.bg, pal.key_hover, hover));
    }
    let color = mix(pal.dim, pal.text, hover);
    let c = rect.center();
    let r = size * 0.27;
    let p = ui.painter();
    p.circle_stroke(c, r, egui::Stroke::new(1.4_f32, color));
    // Right half filled.
    let pts: Vec<egui::Pos2> = (0..=16)
        .map(|i| {
            let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * (i as f32 / 16.0);
            egui::pos2(c.x + r * a.cos(), c.y + r * a.sin())
        })
        .collect();
    p.add(egui::Shape::convex_polygon(pts, color, egui::Stroke::NONE));
    resp.on_hover_cursor(CursorIcon::PointingHand)
}
