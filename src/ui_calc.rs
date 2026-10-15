//! Calculate mode: history, input line and keypad.

use crate::engine::{self, Env, Value};
use crate::fmt::{self, Fmt};
use crate::help;
use crate::sheet;
use crate::theme::{self, Pal};
use crate::widgets::{self, KeyKind};
use eframe::egui::{
    self, pos2, vec2, Align, CursorIcon, Id, Key, Label, Layout, Rect, RichText, ScrollArea, Sense, TextEdit, Ui,
    UiBuilder,
};

pub struct Entry {
    pub expr: String,
    pub value: Value,
}

#[derive(Default)]
pub struct CalcState {
    pub input: String,
    pub history: Vec<Entry>,
    pub error: Option<String>,
    pub shift: bool,
    pub greek: bool,
    /// The physical-constants layer (2nd + αβγ): the scientific keys show c, h, kB, …
    pub consts: bool,
    pub prefer_exact: bool,
    /// Set from other modes to put the caret at the end of the input on the next Calculate frame.
    pub pending_cursor_end: bool,
    /// The sheet view: a text of calculations, one per line, instead of the input line.
    pub sheet: bool,
    pub sheet_text: String,
    pub sheet_path: Option<std::path::PathBuf>,
    /// The text as last opened or saved, to show whether there are unsaved changes.
    pub sheet_saved: String,
    /// What each line came to, from the last run.
    pub sheet_lines: Vec<sheet::Line>,
    /// A note under the sheet's name: a file that could not be read or written.
    pub sheet_note: Option<String>,
    recall: Option<usize>,
    content_h: f32,
    /// When a result was last copied (seconds, egui time), for the short "Copied" note.
    copied_at: Option<f64>,
}

impl CalcState {
    /// The expressions in the history, oldest first (what gets saved between launches).
    pub fn history_exprs(&self) -> Vec<String> {
        self.history.iter().map(|e| e.expr.clone()).collect()
    }

    /// Re-run saved expressions so the history, Ans and stored variables come back.
    pub fn restore(&mut self, exprs: &[String], env: &mut Env) {
        for src in exprs {
            if let Ok(value) = engine::evaluate(src, env) {
                self.history.push(Entry { expr: src.clone(), value });
            }
        }
    }

    /// Add a value from another mode at the end of the input. When the input already ends in a
    /// number or name, a × goes in between so the two are not run together.
    pub fn append(&mut self, text: &str) {
        let joins = self
            .input
            .trim_end()
            .chars()
            .last()
            .map(|c| c.is_alphanumeric() || matches!(c, ')' | ']' | '}' | '!' | '%' | '.') || "⁰¹²³⁴⁵⁶⁷⁸⁹".contains(c))
            .unwrap_or(false);
        if joins {
            self.input.push('×');
        }
        self.input.push_str(text);
        self.error = None;
        self.pending_cursor_end = true;
    }

    /// Copy the latest result to the clipboard (Edit ▸ Copy Last Result).
    pub fn copy_last(&mut self, ctx: &egui::Context) {
        if let Some(e) = self.history.last() {
            ctx.copy_text(fmt::plain(&e.value));
            self.copied_at = Some(ctx.input(|i| i.time));
        }
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
        self.recall = None;
    }

    /// Show a sheet (new, from a template or from a file) in the sheet view.
    pub fn open_sheet(&mut self, text: String, path: Option<std::path::PathBuf>) {
        self.sheet_saved = text.clone();
        self.sheet_text = text;
        self.sheet_path = path;
        self.sheet_note = None;
        self.sheet = true;
        self.pending_cursor_end = true;
    }

    /// The sheet's name for the title line and the save dialog.
    pub fn sheet_name(&self) -> String {
        self.sheet_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".into())
    }

    pub fn sheet_dirty(&self) -> bool {
        self.sheet_text != self.sheet_saved
    }

    /// The sheet for a CSV file: a header and one row per line.
    pub fn sheet_csv(&self) -> (Vec<String>, Vec<Vec<String>>) {
        (vec!["line".into(), "result".into()], sheet::csv_rows(&self.sheet_text, &self.sheet_lines))
    }

    /// The history for a CSV file, oldest first.
    pub fn history_csv(&self) -> (Vec<String>, Vec<Vec<String>>) {
        (vec!["calculation".into(), "result".into()], self.history.iter().map(|e| vec![e.expr.clone(), fmt::plain(&e.value)]).collect())
    }
}

pub fn input_id() -> Id {
    Id::new("calc_input")
}

pub fn sheet_id() -> Id {
    Id::new("calc_sheet")
}

/// Typographic operators and raised powers in the calculations of the sheet, line by line,
/// leaving the comments alone. The caret stays where the user left it.
fn prettify_sheet(ctx: &egui::Context, id: Id, text: &mut String) {
    let pretty: String = text
        .split('\n')
        .map(|line| {
            let (code, comment) = match line.find('#') {
                Some(i) => (&line[..i], &line[i..]),
                None => (line, ""),
            };
            let code = code.replace('*', "×").replace('/', "÷").replace('-', "−").replace("−>", "→");
            format!("{}{}", fmt::prettify(&code, false), comment)
        })
        .collect::<Vec<_>>()
        .join("\n");
    if pretty == *text {
        return;
    }
    let delta = pretty.chars().count() as isize - text.chars().count() as isize;
    *text = pretty;
    if let Some(mut state) = TextEdit::load_state(ctx, id) {
        if let Some(r) = state.cursor.char_range() {
            let pos = (r.primary.index as isize + delta).max(0) as usize;
            state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(pos))));
            state.store(ctx, id);
        }
    }
}

/// Remove the line the caret is on (the AC key in the sheet view).
fn clear_sheet_line(ctx: &egui::Context, id: Id, text: &mut String) {
    let caret = TextEdit::load_state(ctx, id).and_then(|s| s.cursor.char_range()).map(|r| r.primary.index).unwrap_or(0);
    let byte = text.char_indices().nth(caret).map(|(b, _)| b).unwrap_or(text.len());
    let start = text[..byte].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let end = text[byte..].find('\n').map(|i| byte + i).unwrap_or(text.len());
    text.replace_range(start..end, "");
    widgets::cursor_to_end(ctx, id, &text[..start]);
}

/// The sheet: text on the left, each line's result on the right. Returns true when the text changed.
fn sheet_ui(ui: &mut Ui, st: &mut CalcState, f: Fmt, pal: &Pal, rect: Rect) -> bool {
    use eframe::egui::{CornerRadius, Stroke, StrokeKind};
    let ctx = ui.ctx().clone();
    let id = sheet_id();
    ui.painter().rect_filled(rect, CornerRadius::same(12), pal.surface);
    ui.painter().rect_stroke(rect, CornerRadius::same(12), Stroke::new(1.0_f32, pal.line), StrokeKind::Inside);
    let body = rect.shrink2(vec2(14.0, 8.0));
    let results_w = (body.width() * 0.3).clamp(140.0, 260.0);
    let text_w = body.width() - results_w - 12.0;
    let font = theme::mono(14.0);
    let mut changed = false;
    ui.allocate_new_ui(UiBuilder::new().max_rect(body), |ui| {
        ScrollArea::vertical().id_salt("calc_sheet_scroll").auto_shrink([false, false]).show(ui, |ui| {
            ui.set_min_height(body.height() - 4.0);
            // Lines never wrap, so every row of the text is one line of the sheet. Comments are dimmed.
            let (text_color, dim) = (pal.text, pal.faint);
            let font_l = font.clone();
            let mut layouter = move |ui: &Ui, text: &str, _wrap: f32| {
                let mut job = egui::text::LayoutJob::default();
                job.wrap.max_width = f32::INFINITY;
                for (i, line) in text.split('\n').enumerate() {
                    if i > 0 {
                        job.append("\n", 0.0, egui::TextFormat::simple(font_l.clone(), text_color));
                    }
                    let (code, comment) = match line.find('#') {
                        Some(k) => (&line[..k], &line[k..]),
                        None => (line, ""),
                    };
                    job.append(code, 0.0, egui::TextFormat::simple(font_l.clone(), text_color));
                    if !comment.is_empty() {
                        job.append(comment, 0.0, egui::TextFormat::simple(font_l.clone(), dim));
                    }
                }
                ui.fonts(|fo| fo.layout_job(job))
            };
            let out = TextEdit::multiline(&mut st.sheet_text)
                .id(id)
                .frame(false)
                .font(font.clone())
                .desired_width(text_w)
                .desired_rows(12)
                .lock_focus(true)
                .hint_text(RichText::new("R = 4.7kΩ\nC = 100nF\nfc = 1/(2π·R·C)        # one calculation per line").font(font.clone()).color(pal.faint))
                .layouter(&mut layouter)
                .show(ui);
            out.response.clone().on_hover_text(help::SHEET);
            if out.response.changed() {
                changed = true;
                prettify_sheet(&ctx, id, &mut st.sheet_text);
            }
            // Results, one per row, right-aligned beside the text (clear of the scroll bar).
            let x = body.right() - 12.0;
            let clip = ui.clip_rect();
            for (i, row) in out.galley.rows.iter().enumerate() {
                let y = out.galley_pos.y + row.rect.center().y;
                let (text, color) = match st.sheet_lines.get(i) {
                    Some(sheet::Line::Value(v)) => (display(v, f, st.prefer_exact).0, pal.text),
                    Some(sheet::Line::Error(e)) => (e.clone(), pal.danger),
                    _ => continue,
                };
                let mut job = egui::text::LayoutJob::simple_singleline(text, font.clone(), color);
                job.wrap.max_width = results_w;
                job.wrap.max_rows = 1;
                job.wrap.break_anywhere = true;
                let galley = ui.fonts(|fo| fo.layout_job(job));
                let pos = pos2(x - galley.size().x, y - galley.size().y / 2.0);
                // A backing in the card colour, so a long line never runs into its result.
                let back = Rect::from_min_size(pos, galley.size()).expand2(vec2(6.0, 1.0));
                let p = ui.painter().with_clip_rect(clip);
                p.rect_filled(back, CornerRadius::same(4), pal.surface);
                p.galley(pos, galley, color);
            }
        });
    });
    changed
}

#[derive(Clone, Copy, PartialEq)]
enum Act {
    Ins,
    Clear,
    Back,
    Eval,
    Shift,
    Exact,
    Greek,
}

struct K {
    label: &'static str,
    ins: &'static str,
    label2: &'static str,
    ins2: &'static str,
    kind: KeyKind,
    act: Act,
}

const fn f(label: &'static str, ins: &'static str, label2: &'static str, ins2: &'static str) -> K {
    K { label, ins, label2, ins2, kind: KeyKind::Func, act: Act::Ins }
}
const fn d(label: &'static str) -> K {
    K { label, ins: label, label2: "", ins2: "", kind: KeyKind::Digit, act: Act::Ins }
}
const fn o(label: &'static str, ins: &'static str) -> K {
    K { label, ins, label2: "", ins2: "", kind: KeyKind::Op, act: Act::Ins }
}
const fn o2(label: &'static str, ins: &'static str, label2: &'static str, ins2: &'static str) -> K {
    K { label, ins, label2, ins2, kind: KeyKind::Op, act: Act::Ins }
}
const fn a(label: &'static str, kind: KeyKind, act: Act) -> K {
    K { label, ins: "", label2: "", ins2: "", kind, act }
}

const KEYS: [[K; 10]; 5] = [
    [
        a("2nd", KeyKind::Func, Act::Shift),
        f("x²", "²", "x³", "³"),
        f("xʸ", "^", "ʸ√x", "root("),
        f("√", "√(", "³√", "cbrt("),
        f("(", "(", "", ""),
        f(")", ")", "", ""),
        a("AC", KeyKind::Op, Act::Clear),
        a("⌫", KeyKind::Op, Act::Back),
        o2("%", "%", "∥", " ∥ "),
        o("÷", "÷"),
    ],
    [
        f("sin", "sin(", "sin⁻¹", "asin("),
        f("cos", "cos(", "cos⁻¹", "acos("),
        f("tan", "tan(", "tan⁻¹", "atan("),
        f("ln", "ln(", "eˣ", "e^("),
        f("log", "log(", "10ˣ", "10^("),
        f("x⁻¹", "⁻¹", "mod", "mod("),
        d("7"),
        d("8"),
        d("9"),
        o("×", "×"),
    ],
    [
        f("π", "π", "sinh", "sinh("),
        f("e", "e", "cosh", "cosh("),
        f("i", "i", "tanh", "tanh("),
        f("x!", "!", "Γ", "gamma("),
        f("|x|", "abs(", "arg", "arg("),
        f("EXP", "E", "conj", "conj("),
        d("4"),
        d("5"),
        d("6"),
        o("−", "−"),
    ],
    [
        f("x", "x", "A", "A"),
        f("y", "y", "B", "B"),
        f(",", ",", "C", "C"),
        f("nCr", "nCr(", "gcd", "gcd("),
        f("nPr", "nPr(", "lcm", "lcm("),
        f("STO →", "→", "D", "D"),
        d("1"),
        d("2"),
        d("3"),
        o("+", "+"),
    ],
    [
        f("Ans", "Ans", "round", "round("),
        a("a/b", KeyKind::Func, Act::Exact),
        f("∫dx", "integ(", "Σ", "sum("),
        f("d/dx", "diff(", "Π", "prod("),
        f("solve", "solve(", "Rand", "rand()"),
        K { label: "αβγ", ins: "", label2: "CONST", ins2: "", kind: KeyKind::Func, act: Act::Greek },
        d("0"),
        d("."),
        o("(−)", "−"),
        a("=", KeyKind::Accent, Act::Eval),
    ],
];

const GREEK_LOW: [&str; 24] = [
    "α", "β", "γ", "δ", "ε", "ζ", "η", "θ", "ι", "κ", "λ", "μ", "ν", "ξ", "ο", "π", "ρ", "σ", "τ", "υ", "φ", "χ", "ψ",
    "ω",
];
const GREEK_UP: [&str; 24] = [
    "Α", "Β", "Γ", "Δ", "Ε", "Ζ", "Η", "Θ", "Ι", "Κ", "Λ", "Μ", "Ν", "Ξ", "Ο", "Π", "Ρ", "Σ", "Τ", "Υ", "Φ", "Χ", "Ψ",
    "Ω",
];

/// The constants layer uses the same 24 slots as the Greek layer: (key label, name typed).
/// Names match engine::CONSTANTS, which also holds the values shown in the key's tooltip.
const CONST_KEYS: [(&str, &str); 24] = [
    ("c", "c"), ("g", "g"), ("G", "G"), ("h", "h"), ("ħ", "hbar"), ("qₑ", "qe"),
    ("eV", "eV"), ("mₑ", "me"), ("mₚ", "mp"), ("mₙ", "mn"), ("u", "amu"), ("Nᴀ", "NA"),
    ("kB", "kB"), ("R", "Rgas"), ("ε₀", "eps0"), ("μ₀", "mu0"), ("Z₀", "Z0"), ("σ", "kSB"),
    ("atm", "atm"), ("∞", "inf"), ("∥", " ∥ "), ("dB", "dB("), ("limit", "limit("), ("taylor", "taylor("),
];

fn const_key(r: usize, c: usize) -> Option<(&'static str, &'static str)> {
    if c >= 6 || (r == 0 && c == 0) || (r == 4 && c == 5) {
        return None;
    }
    let idx = r * 6 + c - 1;
    if idx < 24 {
        return Some(CONST_KEYS[idx]);
    }
    Some(match c {
        1 => ("STO →", "→"),
        2 => ("(", "("),
        3 => (")", ")"),
        _ => (",", ","),
    })
}

/// Tooltip for a key of the constants layer.
fn const_tip(name: &str) -> String {
    match engine::CONSTANTS.iter().find(|k| k.0 == name) {
        Some((_, v, unit, what)) if v.is_finite() => format!("{}: {} {}", what, fmt::short(*v, 7), unit),
        Some((_, _, _, what)) => what.to_string(),
        None => match name.trim() {
            "∥" => "Parallel: 10k ∥ 4.7k = 3.197k".into(),
            "dB(" => "dB(ratio) = 10·log₁₀. Also dBv (20·log), invdB, invdBv".into(),
            "limit(" => "limit(expression, x, a): a may be inf".into(),
            "taylor(" => "taylor(expression, x, a, n): the coefficients c₀ … cₙ".into(),
            _ => String::new(),
        },
    }
}

/// What the scientific key at (row, column) shows while the Greek layer is on: (label, inserted text).
/// `None` for the keys that keep their normal job (2nd and αβγ).
fn greek_key(r: usize, c: usize, upper: bool) -> Option<(&'static str, &'static str)> {
    if c >= 6 || (r == 0 && c == 0) || (r == 4 && c == 5) {
        return None;
    }
    let idx = r * 6 + c - 1;
    if idx < 24 {
        let l = if upper { GREEK_UP[idx] } else { GREEK_LOW[idx] };
        return Some((l, l));
    }
    Some(match c {
        1 => ("STO →", "→"),
        2 => ("(", "("),
        3 => (")", ")"),
        _ => (",", ","),
    })
}

fn submit(st: &mut CalcState, env: &mut Env) -> bool {
    let src = fmt::prettify(st.input.trim(), true);
    if src.is_empty() {
        return false;
    }
    match engine::evaluate(&src, env) {
        Ok(value) => {
            st.history.push(Entry { expr: src, value });
            if st.history.len() > 500 {
                st.history.remove(0);
            }
            st.input.clear();
            st.error = None;
            st.recall = None;
            true
        }
        Err(e) => {
            st.error = Some(e);
            st.input = src;
            false
        }
    }
}

/// The two ways a value can be shown: (primary, secondary).
fn display(v: &Value, f: Fmt, prefer_exact: bool) -> (String, Option<String>) {
    let dec = fmt::value(v, f);
    let exact = match v {
        Value::Num(z) if z.im == 0.0 => fmt::exact(z.re),
        _ => None,
    };
    match exact {
        Some(e) if prefer_exact => (e, Some(dec)),
        other => (dec, other),
    }
}

/// Returns true when an expression was evaluated this frame.
pub fn ui(ui: &mut Ui, st: &mut CalcState, env: &mut Env, f: Fmt, pal: &Pal, compact: bool) -> bool {
    let ctx = ui.ctx().clone();
    let id = input_id();
    let avail = ui.available_rect_before_wrap();
    let gap = 6.0;
    let cluster = 16.0;
    let key_h = if compact {
        ((avail.height() * 0.62 - 4.0 * gap) / 5.0).clamp(40.0, 72.0)
    } else {
        ((avail.height() * 0.47 - 4.0 * gap) / 5.0).clamp(36.0, 56.0)
    };
    let pad_h = 5.0 * key_h + 4.0 * gap;
    let pad_rect = Rect::from_min_max(pos2(avail.left(), avail.bottom() - pad_h), avail.max);
    let line_rect = Rect::from_min_max(pos2(avail.left(), pad_rect.top() - 104.0), pos2(avail.right(), pad_rect.top() - 18.0));
    // The full calculator has a row above the history: the sheet's name and the view switch.
    let bar_h = if compact { 0.0 } else { 30.0 };
    let hist_rect = Rect::from_min_max(pos2(avail.left(), avail.top() + bar_h), pos2(avail.right(), line_rect.top() - 6.0));
    ui.allocate_rect(avail, Sense::hover());
    if compact {
        st.sheet = false;
    }
    let sheet = st.sheet;
    let id = if sheet { sheet_id() } else { id };

    let mut evaluated = false;
    let mut insert: Option<String> = None;
    let mut load: Option<String> = None;
    let mut copy: Option<String> = None;

    // ---- the row above: sheet name on the left, History | Sheet on the right
    if !compact {
        let bar = Rect::from_min_size(avail.min, vec2(avail.width(), bar_h));
        ui.allocate_new_ui(UiBuilder::new().max_rect(bar).layout(Layout::right_to_left(Align::Center)), |ui| {
            let mut sel = st.sheet as usize;
            if widgets::segmented(ui, pal, &mut sel, &["History", "Sheet"]) {
                st.sheet = sel == 1;
                st.pending_cursor_end = true;
            }
            ui.add_space(6.0);
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.add_space(4.0);
                if st.sheet {
                    let name = format!("{}{}", st.sheet_name(), if st.sheet_dirty() { " •" } else { "" });
                    ui.label(RichText::new(name).font(theme::medium(13.0)).color(pal.dim)).on_hover_text(
                        "The sheet: one calculation per line, name = value stores a value. File ▾ saves and opens sheets.",
                    );
                    if let Some(note) = &st.sheet_note {
                        ui.add_space(8.0);
                        ui.label(RichText::new(note).size(12.5).color(pal.danger));
                    }
                }
            });
        });
    }

    // ---- the sheet takes the place of the history and the input line
    if sheet {
        let rect = Rect::from_min_max(hist_rect.min, pos2(avail.right(), pad_rect.top() - 10.0));
        let changed = sheet_ui(ui, st, f, pal, rect);
        st.sheet_lines = sheet::run(&st.sheet_text, env);
        evaluated = changed;
    }

    // ---- history
    if !sheet {
    ui.allocate_new_ui(UiBuilder::new().max_rect(hist_rect), |ui| {
        ScrollArea::vertical().id_salt("calc_history").auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
            ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.add_space((hist_rect.height() - st.content_h).max(0.0));
                let start = ui.cursor().top();
                if st.history.is_empty() && st.input.is_empty() {
                    ui.label(RichText::new("Type an expression and press return").color(pal.faint).size(13.5));
                    ui.add_space(8.0);
                }
                let count = st.history.len();
                for (idx, e) in st.history.iter().enumerate() {
                    ui.add_space(10.0);
                    // The newest line slides up and fades in.
                    if idx + 1 == count {
                        let t = crate::anim::appear(ui.ctx(), ui.id().with("newest"), (count, e.expr.as_str()), 0.28);
                        ui.set_opacity(t);
                        ui.add_space((1.0 - t) * 10.0);
                    }
                    let r = ui
                        .add(Label::new(RichText::new(&e.expr).color(pal.dim).size(14.0)).sense(Sense::click()))
                        .on_hover_cursor(CursorIcon::PointingHand);
                    if r.clicked() {
                        load = Some(e.expr.clone());
                    }
                    let (main, alt) = display(&e.value, f, st.prefer_exact);
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::BOTTOM), |ui| {
                            let shown = main.clone();
                            let r = ui
                                .add(Label::new(RichText::new(main).font(theme::regular(21.0))).sense(Sense::click()))
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .on_hover_text("Click to copy. Right-click for more.");
                            if r.clicked() {
                                copy = Some(fmt::plain(&e.value));
                            }
                            r.context_menu(|ui| {
                                if ui.button("Copy result").clicked() {
                                    copy = Some(fmt::plain(&e.value));
                                    ui.close_menu();
                                }
                                if ui.button("Copy as shown").clicked() {
                                    copy = Some(shown.clone());
                                    ui.close_menu();
                                }
                                if ui.button("Copy calculation").clicked() {
                                    copy = Some(format!("{} = {}", e.expr, shown));
                                    ui.close_menu();
                                }
                                if ui.button("Insert into the input").clicked() {
                                    insert = Some(fmt::plain(&e.value));
                                    ui.close_menu();
                                }
                            });
                            if let Some(alt) = alt {
                                ui.label(RichText::new(format!("{}  =", alt)).color(pal.faint).size(14.0));
                            }
                        });
                    });
                }
                ui.add_space(4.0);
                st.content_h = ui.cursor().top() - start;
            });
        });
    });

    // ---- input line
    let n = st.input.chars().count();
    let (long, medium) = if line_rect.width() < 520.0 { (18, 12) } else { (44, 28) };
    let size = if n > long {
        20.0
    } else if n > medium {
        26.0
    } else {
        36.0
    };
    let mut resp_changed = false;
    ui.allocate_new_ui(UiBuilder::new().max_rect(line_rect).layout(Layout::bottom_up(Align::RIGHT)), |ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        // Bottom-up: the preview comes first.
        let now = ui.input(|i| i.time);
        let just_copied = st.copied_at.map(|t| now - t < 2.5).unwrap_or(false);
        if just_copied {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
        }
        let preview = if just_copied {
            RichText::new("Copied").color(pal.accent).size(14.0)
        } else if let Some(err) = &st.error {
            RichText::new(err).color(pal.danger).size(14.0)
        } else if st.input.trim().is_empty() {
            RichText::new(" ").size(14.0)
        } else {
            match engine::evaluate_pure(&st.input, env) {
                Ok(v) => RichText::new(format!("= {}", display(&v, f, st.prefer_exact).0)).color(pal.dim).size(15.0),
                Err(_) => RichText::new(" ").size(14.0),
            }
        };
        ui.add(Label::new(preview).truncate());
        let resp = ui.add(
            TextEdit::singleline(&mut st.input)
                .id(id)
                .frame(false)
                .font(theme::light(size))
                .horizontal_align(Align::RIGHT)
                .desired_width(line_rect.width())
                .text_color(pal.text)
                .margin(egui::Margin::ZERO),
        );
        let resp = resp.on_hover_text(help::EXPR);
        if resp.changed() {
            resp_changed = true;
        }
        if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            evaluated = submit(st, env);
            resp.request_focus();
        }
        if resp.has_focus() && !st.history.is_empty() {
            let (up, down) = ui.input(|i| (i.key_pressed(Key::ArrowUp), i.key_pressed(Key::ArrowDown)));
            if up {
                let idx = st.recall.map(|i| i.saturating_sub(1)).unwrap_or(st.history.len() - 1);
                st.recall = Some(idx);
                load = Some(st.history[idx].expr.clone());
            } else if down {
                if let Some(i) = st.recall {
                    if i + 1 < st.history.len() {
                        st.recall = Some(i + 1);
                        load = Some(st.history[i + 1].expr.clone());
                    } else {
                        st.recall = None;
                        load = Some(String::new());
                    }
                }
            }
        }
        if ui.input(|i| i.key_pressed(Key::Escape)) {
            st.input.clear();
            st.error = None;
        }
    });

    if resp_changed {
        st.error = None;
        // Typing "=" evaluates, like pressing return.
        if st.input.contains('=') {
            st.input = st.input.replace('=', "");
            evaluated = submit(st, env);
        }
        // Typographic operators, one character for one so the cursor stays put.
        if st.input.contains(['*', '/', '-']) && !st.input.contains("->") {
            st.input = st.input.replace('*', "×").replace('/', "÷").replace('-', "−");
        }
        st.input = st.input.replace("−>", "→");
        widgets::prettify_field(&ctx, id, &mut st.input, false);
        // Starting with an operator continues from the last answer.
        if !st.history.is_empty() && ["+", "×", "÷", "^"].contains(&st.input.as_str()) {
            st.input = format!("Ans{}", st.input);
            widgets::cursor_to_end(&ctx, id, &st.input);
        }
    }
    } // !sheet

    // ---- keypad
    let key_w = if compact {
        (pad_rect.width() - 3.0 * gap) / 4.0
    } else {
        (pad_rect.width() - 8.0 * gap - cluster) / 10.0
    };
    for (r, row) in KEYS.iter().enumerate() {
        for (c, k) in row.iter().enumerate() {
            // The simple calculator shows only the four right-hand columns.
            if compact && c < 6 {
                continue;
            }
            let x = if compact {
                pad_rect.left() + (c - 6) as f32 * (key_w + gap)
            } else {
                pad_rect.left() + c as f32 * (key_w + gap) + if c >= 6 { cluster - gap } else { 0.0 }
            };
            let y = pad_rect.top() + r as f32 * (key_h + gap);
            let rect = Rect::from_min_size(pos2(x, y), vec2(key_w, key_h));
            // In the sheet view the keys type into the sheet.
            let target: &mut String = if sheet { &mut st.sheet_text } else { &mut st.input };
            if let Some((label, text)) = if st.consts { const_key(r, c) } else { None } {
                let resp = widgets::key_at(ui, pal, rect, (r, c), label, KeyKind::Func, false);
                let tip = const_tip(text);
                let resp = if tip.is_empty() { resp } else { resp.on_hover_text(tip) };
                if resp.clicked() {
                    widgets::insert_at_cursor(&ctx, id, target, text);
                    st.error = None;
                    st.shift = false;
                }
                continue;
            }
            if let Some((label, text)) = if st.greek { greek_key(r, c, st.shift) } else { None } {
                let resp = widgets::key_at(ui, pal, rect, (r, c), label, KeyKind::Func, false);
                let resp = if label.chars().count() == 1 && label != "(" && label != ")" && label != "," {
                    resp.on_hover_text(if label == "π" {
                        "The constant π"
                    } else {
                        "Greek letter, used as a variable. Give it a value with →, e.g. 5 → α"
                    })
                } else {
                    resp
                };
                if resp.clicked() {
                    widgets::insert_at_cursor(&ctx, id, target, text);
                    st.error = None;
                    st.shift = false;
                }
                continue;
            }
            let shifted = st.shift && !k.label2.is_empty() && !st.greek && !st.consts;
            let label = if shifted { k.label2 } else { k.label };
            let active = (k.act == Act::Shift && st.shift)
                || (k.act == Act::Exact && st.prefer_exact)
                || (k.act == Act::Greek && (st.greek || st.consts));
            let resp = widgets::key_at(ui, pal, rect, (r, c), label, k.kind, active);
            let resp = match help::key_tip(label) {
                Some(tip) => resp.on_hover_text(tip),
                None => resp,
            };
            if !resp.clicked() {
                continue;
            }
            if sheet {
                match k.act {
                    Act::Ins => {
                        widgets::insert_at_cursor(&ctx, id, &mut st.sheet_text, if shifted { k.ins2 } else { k.ins });
                        prettify_sheet(&ctx, id, &mut st.sheet_text);
                        st.shift = false;
                    }
                    Act::Clear => clear_sheet_line(&ctx, id, &mut st.sheet_text),
                    Act::Back => widgets::backspace_at_cursor(&ctx, id, &mut st.sheet_text),
                    Act::Eval => widgets::insert_at_cursor(&ctx, id, &mut st.sheet_text, " = "),
                    Act::Shift => st.shift = !st.shift,
                    Act::Exact => st.prefer_exact = !st.prefer_exact,
                    Act::Greek => {
                        if shifted {
                            st.consts = !st.consts;
                            st.greek = false;
                        } else if st.consts {
                            st.consts = false;
                        } else {
                            st.greek = !st.greek;
                        }
                        st.shift = false;
                    }
                }
                ctx.memory_mut(|m| m.request_focus(id));
                continue;
            }
            match k.act {
                Act::Ins => {
                    let text = if shifted { k.ins2 } else { k.ins };
                    let continues = st.input.is_empty() && !st.history.is_empty();
                    if continues && ["+", "×", "÷", "^", "²", "³", "⁻¹", "!", "%", "→"].contains(&text) {
                        st.input = "Ans".into();
                        widgets::cursor_to_end(&ctx, id, &st.input);
                    }
                    widgets::insert_at_cursor(&ctx, id, &mut st.input, text);
                    widgets::prettify_field(&ctx, id, &mut st.input, false);
                    st.error = None;
                    st.shift = false;
                }
                Act::Clear => {
                    if st.input.is_empty() && st.error.is_none() {
                        st.clear_history();
                    }
                    st.input.clear();
                    st.error = None;
                    ctx.memory_mut(|m| m.request_focus(id));
                }
                Act::Back => {
                    widgets::backspace_at_cursor(&ctx, id, &mut st.input);
                    st.error = None;
                }
                Act::Eval => {
                    evaluated = submit(st, env);
                    ctx.memory_mut(|m| m.request_focus(id));
                }
                Act::Shift => st.shift = !st.shift,
                Act::Exact => st.prefer_exact = !st.prefer_exact,
                Act::Greek => {
                    // αβγ toggles the Greek layer; 2nd + αβγ (shown as CONST) the constants layer.
                    if shifted {
                        st.consts = !st.consts;
                        st.greek = false;
                    } else if st.consts {
                        st.consts = false;
                    } else {
                        st.greek = !st.greek;
                    }
                    st.shift = false;
                }
            }
        }
    }

    // ⌘⇧C (Ctrl+Shift+C) copies the latest result.
    // On macOS ⌘C arrives as a Copy event rather than a key press, so accept either.
    let copy_last = ctx.input(|i| {
        i.modifiers.command
            && i.modifiers.shift
            && (i.key_pressed(Key::C) || i.events.iter().any(|e| matches!(e, egui::Event::Copy)))
    });
    if copy_last {
        if let Some(e) = st.history.last() {
            copy = Some(fmt::plain(&e.value));
        }
    }
    if let Some(text) = copy {
        ctx.copy_text(text);
        st.copied_at = Some(ctx.input(|i| i.time));
    }
    if let Some(text) = load {
        st.input = text;
        st.error = None;
        widgets::cursor_to_end(&ctx, id, &st.input);
    }
    if let Some(text) = insert {
        widgets::insert_at_cursor(&ctx, id, &mut st.input, &text);
        st.error = None;
    }
    if st.pending_cursor_end {
        st.pending_cursor_end = false;
        widgets::cursor_to_end(&ctx, id, if sheet { &st.sheet_text } else { &st.input });
    }
    // Keep the input focused so typing always lands in it.
    if ctx.memory(|m| m.focused().is_none()) {
        ctx.memory_mut(|m| m.request_focus(id));
    }
    evaluated
}
