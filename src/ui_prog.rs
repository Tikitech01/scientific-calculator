//! Programmer mode: one integer shown in every base, a clickable bit grid and bitwise operations.

use crate::engine::{self, Env};
use crate::fmt;
use crate::help;
use crate::theme::{self, mix, Pal};
use crate::widgets;
use eframe::egui::{self, pos2, vec2, Align2, CornerRadius, CursorIcon, Rect, RichText, ScrollArea, Sense, Ui};

const WORDS: [u32; 4] = [8, 16, 32, 64];
const ROWS: [(&str, u32); 4] = [("HEX", 16), ("DEC", 10), ("OCT", 8), ("BIN", 2)];

pub struct ProgState {
    pub value: u64,
    pub word: usize,
    pub signed: bool,
    text: [String; 4],
    focus: Option<usize>,
    error: Option<usize>,
    pub operand: String,
    op_error: Option<String>,
}

impl Default for ProgState {
    fn default() -> Self {
        ProgState {
            value: 0,
            word: 3,
            signed: true,
            text: Default::default(),
            focus: None,
            error: None,
            operand: String::new(),
            op_error: None,
        }
    }
}

impl ProgState {
    fn bits(&self) -> u32 {
        WORDS[self.word]
    }

    fn mask(&self) -> u64 {
        if self.bits() == 64 {
            u64::MAX
        } else {
            (1u64 << self.bits()) - 1
        }
    }

    /// The value read as a signed number of the current word size.
    fn signed_value(&self) -> i64 {
        let b = self.bits();
        if b == 64 {
            self.value as i64
        } else {
            let shift = 64 - b;
            ((self.value << shift) as i64) >> shift
        }
    }

    fn format(&self, radix: u32) -> String {
        if radix == 10 {
            return if self.signed { self.signed_value().to_string() } else { self.value.to_string() };
        }
        let raw = fmt::to_base(self.value as i64, radix);
        let group = match radix {
            2 => 4,
            16 => 4,
            _ => 0,
        };
        if group == 0 {
            return raw;
        }
        // Pad binary to whole nibbles and group digits in fours from the right.
        let raw = if radix == 2 {
            let width = raw.len().div_ceil(4) * 4;
            format!("{:0>width$}", raw, width = width)
        } else {
            raw
        };
        let ch: Vec<char> = raw.chars().collect();
        let mut out = String::new();
        for (i, c) in ch.iter().enumerate() {
            if i > 0 && (ch.len() - i) % group == 0 {
                out.push(' ');
            }
            out.push(*c);
        }
        out
    }

    fn parse(&self, text: &str, radix: u32) -> Option<u64> {
        let t: String = text.chars().filter(|c| !c.is_whitespace() && *c != '_' && *c != ',').collect();
        if t.is_empty() {
            return Some(0);
        }
        let v = if radix == 10 {
            let t = t.replace('−', "-");
            match t.parse::<i64>() {
                Ok(v) => v as u64,
                Err(_) => t.parse::<u64>().ok()?,
            }
        } else {
            let t = t.trim_start_matches("0x").trim_start_matches("0b").trim_start_matches("0o");
            u64::from_str_radix(t, radix).ok()?
        };
        Some(v & self.mask())
    }
}

fn apply(st: &mut ProgState, op: &str, env: &mut Env) {
    st.op_error = None;
    let a = st.value;
    let b = if op == "NOT" || op == "NEG" {
        0
    } else {
        let src = st.operand.trim();
        let parsed = if let Some(h) = src.strip_prefix("0x") {
            u64::from_str_radix(h, 16).ok()
        } else if let Some(b) = src.strip_prefix("0b") {
            u64::from_str_radix(b, 2).ok()
        } else {
            match engine::field_value(src, env) {
                Ok(Some(v)) if v.fract() == 0.0 => Some(v as i64 as u64),
                _ => None,
            }
        };
        match parsed {
            Some(v) => v & st.mask(),
            None => {
                st.op_error = Some("Enter a whole number for B (decimal, 0x… or 0b…)".into());
                return;
            }
        }
    };
    let bits = st.bits();
    let sh = (b % bits as u64) as u32;
    let r = match op {
        "AND" => a & b,
        "OR" => a | b,
        "XOR" => a ^ b,
        "NAND" => !(a & b),
        "NOR" => !(a | b),
        "NOT" => !a,
        "NEG" => a.wrapping_neg(),
        "<<" => {
            if b >= bits as u64 {
                0
            } else {
                a << b
            }
        }
        ">>" => {
            if b >= bits as u64 {
                0
            } else {
                a >> b
            }
        }
        "ROL" => ((a << sh) | if sh == 0 { 0 } else { a >> (bits - sh) }) & st.mask(),
        "ROR" => ((a >> sh) | if sh == 0 { 0 } else { a << (bits - sh) }) & st.mask(),
        "+" => a.wrapping_add(b),
        "−" => a.wrapping_sub(b),
        "×" => a.wrapping_mul(b),
        "÷" | "mod" => {
            if b == 0 {
                st.op_error = Some("Division by zero".into());
                return;
            }
            let signed = st.signed;
            let sv = |x: u64| -> i64 {
                let shift = 64 - bits;
                if bits == 64 {
                    x as i64
                } else {
                    ((x << shift) as i64) >> shift
                }
            };
            if signed {
                let (x, y) = (sv(a), sv(b));
                (if op == "÷" { x.wrapping_div(y) } else { x.wrapping_rem(y) }) as u64
            } else if op == "÷" {
                a / b
            } else {
                a % b
            }
        }
        _ => a,
    };
    st.value = r & st.mask();
}

/// Returns text to insert into the Calculate input, if the user asked for it.
pub fn ui(ui: &mut Ui, st: &mut ProgState, env: &mut Env, pal: &Pal) -> Option<String> {
    let mut send = None;
    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.set_max_width(980.0);
        ui.horizontal(|ui| {
            widgets::dim(ui, pal, "Word size");
            let before = st.word;
            widgets::segmented(ui, pal, &mut st.word, &["8-bit", "16-bit", "32-bit", "64-bit"]);
            if st.word != before {
                st.value &= st.mask();
            }
            ui.add_space(12.0);
            let mut s = if st.signed { 0 } else { 1 };
            if widgets::segmented(ui, pal, &mut s, &["Signed", "Unsigned"]) {
                st.signed = s == 0;
            }
        });
        ui.add_space(14.0);

        // ---- the value in every base; type in any row
        let mut new_focus = None;
        for (i, (name, radix)) in ROWS.iter().enumerate() {
            if st.focus != Some(i) {
                // Zero shows as the grey hint, so typing starts from an empty field.
                st.text[i] = if st.value == 0 { String::new() } else { st.format(*radix) };
            }
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(44.0, 32.0), Sense::hover());
                ui.painter().text(r.left_center(), Align2::LEFT_CENTER, *name, theme::medium(12.5), pal.dim);
                let w = (ui.available_width() - 60.0).min(640.0);
                let resp = widgets::field_with(ui, pal, &mut st.text[i], "0", w, egui::Align::LEFT, Some(egui::Id::new(("prog_row", i))))
                    .on_hover_text(match radix {
                        16 => "Type a hex number (0–9, A–F). The other rows follow.",
                        10 => "Type a whole number. Negative numbers use two's complement.",
                        8 => "Type an octal number (0–7).",
                        _ => "Type a binary number (0 and 1). Spaces are ignored.",
                    });
                if resp.has_focus() {
                    new_focus = Some(i);
                }
                if resp.changed() {
                    match st.parse(&st.text[i], *radix) {
                        Some(v) => {
                            st.value = v;
                            st.error = None;
                        }
                        None => st.error = Some(i),
                    }
                }
                if st.error == Some(i) {
                    widgets::error(ui, pal, "Not valid");
                }
            });
        }
        st.focus = new_focus;
        if st.focus.is_none() {
            st.error = None;
        }
        // The row being typed in is left alone above. If a bit click or an operation below
        // changes the value, give up that focus so every row shows the new value next frame.
        let typed_value = st.value;

        ui.add_space(16.0);
        // Bits on the left and operations on the right, so everything fits without scrolling.
        ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
        // ---- bit grid, most significant bit first, 16 bits per row
        widgets::caption(ui, pal, "BITS  ·  click a bit to flip it");
        ui.add_space(6.0);
        let bits = st.bits();
        let per_row = bits.min(16);
        let rows = bits / per_row;
        let cell = 26.0f32;
        let gap = 3.0f32;
        let nib_gap = 9.0f32;
        let row_w = per_row as f32 * (cell + gap) + (per_row / 4 - 1) as f32 * nib_gap;
        for row in 0..rows {
            let (rect, _) = ui.allocate_exact_size(vec2(row_w + 8.0, cell + 16.0), Sense::hover());
            for k in 0..per_row {
                let bit = bits - 1 - (row * per_row + k);
                let x = rect.left() + k as f32 * (cell + gap) + (k / 4) as f32 * nib_gap;
                let r = Rect::from_min_size(pos2(x, rect.top()), vec2(cell, cell));
                let resp = ui.interact(r, ui.id().with(("bit", bit)), Sense::click());
                let on = st.value >> bit & 1 == 1;
                let hover = crate::anim::hover(ui.ctx(), resp.id, resp.hovered());
                let fill = if on { pal.accent } else { mix(pal.key, pal.key_hover, hover) };
                ui.painter().rect_filled(r, CornerRadius::same(6), fill);
                ui.painter().text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    if on { "1" } else { "0" },
                    theme::medium(12.5),
                    if on { pal.on_accent } else { pal.dim },
                );
                if k % 4 == 3 || k == 0 {
                    ui.painter().text(
                        pos2(r.center().x, r.bottom() + 8.0),
                        Align2::CENTER_CENTER,
                        bit.to_string(),
                        theme::regular(10.0),
                        pal.faint,
                    );
                }
                if resp.on_hover_cursor(CursorIcon::PointingHand).on_hover_text(format!("Bit {}", bit)).clicked() {
                    st.value ^= 1u64 << bit;
                }
            }
        }

        });
        ui.add_space(28.0);
        ui.vertical(|ui| {
        // ---- operations
        widgets::caption(ui, pal, "OPERATIONS  ·  the result replaces the value");
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            widgets::dim(ui, pal, "B");
            widgets::field(ui, pal, &mut st.operand, "e.g. 0xFF, 0b1010, 12", 180.0)
                .on_hover_text("The second number for the operation: decimal, 0x… for hex or 0b… for binary. Calculations work too.");
        });
        if let Some(e) = &st.op_error {
            widgets::error(ui, pal, e.clone());
        }
        ui.add_space(8.0);
        let groups: [&[(&str, &str)]; 3] = [
            &[
                ("AND", "Bitwise AND"),
                ("OR", "Bitwise OR"),
                ("XOR", "Bitwise exclusive OR"),
                ("NAND", "NOT (value AND B)"),
                ("NOR", "NOT (value OR B)"),
                ("NOT", "Flip every bit of the value (B not used)"),
            ],
            &[
                ("<<", "Shift left by B bits"),
                (">>", "Shift right by B bits (logical)"),
                ("ROL", "Rotate left by B bits"),
                ("ROR", "Rotate right by B bits"),
                ("NEG", "Two's complement negation (B not used)"),
            ],
            &[
                ("+", "Add B (wraps at the word size)"),
                ("−", "Subtract B"),
                ("×", "Multiply by B"),
                ("÷", "Integer division by B"),
                ("mod", "Remainder after division by B"),
            ],
        ];
        for g in groups {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for (op, tip) in g {
                    let kind = widgets::KeyKind::Op;
                    if widgets::key(ui, pal, op, kind, vec2(58.0, 34.0), false).on_hover_text(*tip).clicked() {
                        apply(st, op, env);
                    }
                }
            });
            ui.add_space(4.0);
        }

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if widgets::link(ui, pal, "Clear") {
                st.value = 0;
            }
            ui.add_space(8.0);
            if widgets::link(ui, pal, "Use Ans") {
                if let Some(v) = env.ans_real() {
                    if v.fract() == 0.0 {
                        st.value = (v as i64 as u64) & st.mask();
                    }
                }
            }
            ui.add_space(8.0);
            if widgets::link(ui, pal, "Send to Calculate") {
                send = Some(if st.signed { st.signed_value().to_string() } else { st.value.to_string() });
            }
        });
        });
        });
        ui.add_space(8.0);
        widgets::dim(ui, pal, help::PROG_NOTE);
        let _ = RichText::new("");
        if st.value != typed_value {
            st.focus = None;
            for i in 0..ROWS.len() {
                ui.memory_mut(|m| m.surrender_focus(egui::Id::new(("prog_row", i))));
            }
        }
    });
    send
}
