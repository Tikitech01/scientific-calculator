//! Graph and Table modes. Both share one list of functions.

use crate::engine::{self, Env, Expr};
use crate::fmt;
use crate::help;
use crate::numeric::{self, Extremum};
use crate::plot::{self, View};
use crate::theme::{self, mix, Pal};
use crate::widgets;
use eframe::egui::{
    self, pos2, vec2, Align, Align2, Color32, CornerRadius, CursorIcon, Layout, Pos2, Rect, RichText, ScrollArea,
    Sense, Stroke, StrokeKind, Ui, UiBuilder,
};

pub enum Kind {
    Cart(Expr),
    Polar(Expr),
    Param(Expr, Expr),
    /// y < f(x) / y > f(x): the region is shaded. `strict` draws the boundary dashed.
    Ineq { expr: Expr, above: bool, strict: bool },
}

pub struct FnEntry {
    pub text: String,
    pub visible: bool,
    /// Also draw the derivative f′(x), dashed.
    pub show_deriv: bool,
    parsed_for: Option<String>,
    pub kind: Option<Result<Kind, String>>,
}

/// A free letter in a function (a, b, k …) that gets a slider.
pub struct Param {
    pub name: String,
    pub value: f64,
    pub min: String,
    pub max: String,
    pub animate: bool,
}

impl FnEntry {
    pub fn new(text: &str) -> Self {
        FnEntry { text: text.into(), visible: true, show_deriv: false, parsed_for: None, kind: None }
    }

    fn refresh(&mut self) {
        if self.parsed_for.as_deref() == Some(self.text.as_str()) {
            return;
        }
        self.parsed_for = Some(self.text.clone());
        let mut t = self.text.trim();
        for prefix in ["y=", "y =", "r=", "r =", "f(x)=", "f(x) ="] {
            if let Some(rest) = t.strip_prefix(prefix) {
                t = rest.trim();
            }
        }
        if t.is_empty() {
            self.kind = None;
            return;
        }
        // Inequalities: y < f(x), y ≥ f(x) …
        for (prefix, above, strict) in [
            ("y<=", false, false), ("y≤", false, false), ("y>=", true, false), ("y≥", true, false),
            ("y<", false, true), ("y>", true, true),
        ] {
            let compact: String = t.chars().filter(|c| !c.is_whitespace()).take(3).collect();
            if compact.starts_with(prefix) {
                let rest = t.trim_start_matches('y').trim_start().trim_start_matches(['<', '>', '=', '≤', '≥']).trim();
                self.kind = Some(engine::parse(rest).map(|expr| Kind::Ineq { expr, above, strict }));
                return;
            }
        }
        self.kind = Some(parse_kind(t));
    }

    /// The y = f(x) expression, for plain curves and the boundary of an inequality.
    pub fn cart(&self) -> Option<&Expr> {
        match &self.kind {
            Some(Ok(Kind::Cart(e))) => Some(e),
            Some(Ok(Kind::Ineq { expr, .. })) => Some(expr),
            _ => None,
        }
    }

    fn expr(&self) -> Option<&Expr> {
        match &self.kind {
            Some(Ok(Kind::Cart(e))) | Some(Ok(Kind::Polar(e))) => Some(e),
            Some(Ok(Kind::Ineq { expr, .. })) => Some(expr),
            _ => None,
        }
    }
}

/// "(x(t), y(t))" → parametric; anything using θ → polar; otherwise y = f(x).
fn parse_kind(t: &str) -> Result<Kind, String> {
    if t.starts_with('(') && t.ends_with(')') {
        let inner = &t[1..t.len() - 1];
        let mut depth = 0i32;
        let mut split = None;
        let mut balanced = true;
        for (i, c) in inner.char_indices() {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => {
                    depth -= 1;
                    if depth < 0 {
                        balanced = false;
                        break;
                    }
                }
                ',' if depth == 0 && split.is_none() => split = Some(i),
                _ => {}
            }
        }
        if let (true, Some(i)) = (balanced, split) {
            let x = engine::parse(&inner[..i])?;
            let y = engine::parse(&inner[i + 1..])?;
            return Ok(Kind::Param(x, y));
        }
    }
    let e = engine::parse(t)?;
    if engine::uses_var(&e, "θ") {
        Ok(Kind::Polar(e))
    } else {
        Ok(Kind::Cart(e))
    }
}

#[derive(Default)]
struct Analysis {
    roots: Vec<f64>,
    extrema: Vec<Extremum>,
    y_intercept: Option<f64>,
    /// Between every pair of visible y = f(x) curves: (first index, second index, x, y).
    intersections: Vec<(usize, usize, f64, f64)>,
    integral: Option<Result<(f64, f64, f64), String>>,
}

pub struct GraphState {
    pub fns: Vec<FnEntry>,
    pub sel: usize,
    pub view: View,
    inited: bool,
    pub int_a: String,
    pub int_b: String,
    pub t_min: String,
    pub t_max: String,
    analysis: Analysis,
    /// What the analysis was computed for, so it is only redone when something changed.
    analysis_key: String,
    // Table mode
    pub tb_start: String,
    pub tb_end: String,
    pub tb_step: String,
    /// PNG export: 1 = hide the controls this frame, 2 = screenshot requested, waiting for it.
    export: u8,
    export_rect: Rect,
    export_msg: Option<(String, f64)>,
    /// Sliders for the free letters used in the functions.
    pub params: Vec<Param>,
    /// Show a tangent line where the mouse traces the selected curve.
    pub tangent: bool,
    /// The Statistics X/Y columns, drawn as points when `show_points` is on.
    pub points: Vec<(f64, f64)>,
    pub show_points: bool,
}

impl Default for GraphState {
    fn default() -> Self {
        GraphState {
            fns: vec![FnEntry::new(""), FnEntry::new("")],
            sel: 0,
            view: View::default(),
            inited: false,
            int_a: String::new(),
            int_b: String::new(),
            t_min: String::new(),
            t_max: String::new(),
            analysis: Analysis::default(),
            analysis_key: String::new(),
            tb_start: "-5".into(),
            tb_end: "5".into(),
            tb_step: "1".into(),
            export: 0,
            export_rect: Rect::NOTHING,
            export_msg: None,
            params: Vec::new(),
            tangent: false,
            points: Vec::new(),
            show_points: false,
        }
    }
}

const PANEL_W: f32 = 290.0;

fn split(ui: &mut Ui) -> (Rect, Rect) {
    let avail = ui.available_rect_before_wrap();
    ui.allocate_rect(avail, Sense::hover());
    let left = Rect::from_min_max(avail.min, pos2(avail.left() + PANEL_W, avail.bottom()));
    let right = Rect::from_min_max(pos2(left.right() + 18.0, avail.top()), avail.max);
    (left, right)
}

/// The editable list of functions.
fn function_list(ui: &mut Ui, st: &mut GraphState, pal: &Pal) {
    widgets::caption(ui, pal, "FUNCTIONS");
    ui.add_space(2.0);
    let mut remove = None;
    let count = st.fns.len();
    for (i, f) in st.fns.iter_mut().enumerate() {
        f.refresh();
        let color = pal.series[i % pal.series.len()];
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let (dot, resp) = ui.allocate_exact_size(vec2(18.0, 30.0), Sense::click());
            if f.visible {
                ui.painter().circle_filled(dot.center(), 5.5, color);
            } else {
                ui.painter().circle_stroke(dot.center(), 5.0, Stroke::new(1.5_f32, pal.faint));
            }
            if resp.on_hover_cursor(CursorIcon::PointingHand).on_hover_text("Show or hide").clicked() {
                f.visible = !f.visible;
            }
            let prefix = match &f.kind {
                Some(Ok(Kind::Polar(_))) => "r =",
                Some(Ok(Kind::Param(..))) => "x, y",
                Some(Ok(Kind::Ineq { above: true, .. })) => "y >",
                Some(Ok(Kind::Ineq { .. })) => "y <",
                _ => "y =",
            };
            let (pr, _) = ui.allocate_exact_size(vec2(26.0, 30.0), Sense::hover());
            ui.painter().text(pr.left_center(), Align2::LEFT_CENTER, prefix, theme::regular(13.0), pal.dim);
            let w = ui.available_width() - 18.0 - 22.0 - if count > 1 { 28.0 } else { 0.0 };
            let hint = match i {
                0 => "x² − 2x − 3",
                1 => "sin(x)",
                _ => "",
            };
            let r = widgets::field(ui, pal, &mut f.text, hint, w.max(60.0)).on_hover_text(help::FUNCTION);
            if r.has_focus() || r.clicked() {
                st.sel = i;
            }
            if count > 1 && widgets::glyph(ui, pal, "×", 22.0).on_hover_text("Remove").clicked() {
                remove = Some(i);
            }
        });
        if let Some(Err(e)) = &f.kind {
            ui.horizontal(|ui| {
                ui.add_space(56.0);
                widgets::error(ui, pal, e.clone());
            });
        }
    }
    if let Some(i) = remove {
        st.fns.remove(i);
        st.sel = st.sel.min(st.fns.len() - 1);
    }
    if st.fns.len() < 6 {
        ui.add_space(2.0);
        if widgets::link(ui, pal, "+  Add function") {
            st.fns.push(FnEntry::new(""));
            st.sel = st.fns.len() - 1;
        }
    }
}

/// Letters that are never parameters: the graph variables and built-in names.
const RESERVED: [&str; 8] = ["x", "t", "θ", "theta", "e", "i", "pi", "Ans"];

/// Keep the slider list in step with the letters used in the functions.
fn refresh_params(st: &mut GraphState, env: &Env) {
    let mut names = Vec::new();
    for f in &st.fns {
        match &f.kind {
            Some(Ok(Kind::Param(a, b))) => {
                engine::var_names(a, &mut names);
                engine::var_names(b, &mut names);
            }
            _ => {
                if let Some(e) = f.expr() {
                    engine::var_names(e, &mut names);
                }
            }
        }
    }
    names.retain(|n| !RESERVED.contains(&n.as_str()) && n.chars().count() == 1 && n.chars().all(|c| c.is_alphabetic()));
    st.params.retain(|p| names.contains(&p.name));
    for n in names {
        if !st.params.iter().any(|p| p.name == n) {
            let value = match env.vars.get(&n) {
                Some(engine::Value::Num(z)) if z.im == 0.0 && z.re.is_finite() => z.re,
                _ => 1.0,
            };
            st.params.push(Param { name: n, value, min: "-5".into(), max: "5".into(), animate: false });
        }
    }
}

/// Sliders for the parameters, which are stored as variables so Calculate sees them too.
fn params_panel(ui: &mut Ui, st: &mut GraphState, env: &mut Env, pal: &Pal) {
    if st.params.is_empty() {
        return;
    }
    ui.add_space(14.0);
    widgets::caption(ui, pal, "SLIDERS");
    ui.label(RichText::new("Letters in the functions become sliders. Try a·x² + b.").size(12.0).color(pal.faint));
    ui.add_space(4.0);
    let dt = ui.input(|i| i.stable_dt).min(0.1) as f64;
    for p in st.params.iter_mut() {
        let lo = engine::field_value(&p.min, env).ok().flatten().unwrap_or(-5.0);
        let hi = engine::field_value(&p.max, env).ok().flatten().unwrap_or(5.0);
        let (lo, hi) = if lo < hi { (lo, hi) } else { (hi, lo + 1.0) };
        if p.animate {
            let span = hi - lo;
            p.value += span * dt / 4.0;
            if p.value > hi {
                p.value = lo;
            }
            ui.ctx().request_repaint();
        }
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let (r, _) = ui.allocate_exact_size(vec2(16.0, 24.0), Sense::hover());
            ui.painter().text(r.left_center(), Align2::LEFT_CENTER, &p.name, theme::medium(14.0), pal.text);
            ui.spacing_mut().slider_width = 118.0;
            ui.add(egui::Slider::new(&mut p.value, lo..=hi).show_value(false).trailing_fill(true));
            ui.add(
                egui::Label::new(RichText::new(fmt::short(p.value, 4)).size(13.0)).selectable(true),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let glyph = if p.animate { "■" } else { "▶" };
                if widgets::glyph(ui, pal, glyph, 22.0).on_hover_text(if p.animate { "Stop" } else { "Animate: sweep from min to max" }).clicked() {
                    p.animate = !p.animate;
                }
            });
        });
        ui.horizontal(|ui| {
            ui.add_space(22.0);
            ui.spacing_mut().item_spacing.x = 6.0;
            widgets::field(ui, pal, &mut p.min, "min", 50.0).on_hover_text("Smallest slider value");
            widgets::dim(ui, pal, "to");
            widgets::field(ui, pal, &mut p.max, "max", 50.0).on_hover_text("Largest slider value");
        });
        env.set_real(&p.name, p.value);
    }
}

fn point(x: f64, y: f64) -> String {
    format!("({}, {})", fmt::short(x, 7), fmt::short(y, 7))
}

fn analyse(st: &GraphState, env: &mut Env, x0: f64, x1: f64) -> Analysis {
    let mut a = Analysis::default();
    let n = 600;
    let carts: Vec<(usize, &Expr)> =
        st.fns.iter().enumerate().filter(|(_, f)| f.visible).filter_map(|(i, f)| f.cart().map(|e| (i, e))).collect();
    for (k, (i, f)) in carts.iter().enumerate() {
        for (j, g) in carts.iter().skip(k + 1) {
            let xs = numeric::find_roots(
                &mut |x| engine::eval_at(f, env, "x", x) - engine::eval_at(g, env, "x", x),
                x0,
                x1,
                n,
            );
            for x in xs {
                let y = engine::eval_at(f, env, "x", x);
                if y.is_finite() {
                    a.intersections.push((*i, *j, x, y));
                }
            }
        }
    }
    let Some(expr) = st.fns.get(st.sel).and_then(|f| f.cart()) else {
        return a;
    };
    a.roots = numeric::find_roots(&mut |x| engine::eval_at(expr, env, "x", x), x0, x1, n);
    a.extrema = numeric::find_extrema(&mut |x| engine::eval_at(expr, env, "x", x), x0, x1, n);
    let y0 = engine::eval_at(expr, env, "x", 0.0);
    if y0.is_finite() {
        a.y_intercept = Some(y0);
    }
    if !st.int_a.trim().is_empty() && !st.int_b.trim().is_empty() {
        let bounds = (|| -> Result<(f64, f64), String> {
            let lo = engine::field_value(&st.int_a, env)?.ok_or("Enter both limits")?;
            let hi = engine::field_value(&st.int_b, env)?.ok_or("Enter both limits")?;
            Ok((lo, hi))
        })();
        a.integral = Some(bounds.and_then(|(lo, hi)| {
            let v = numeric::integrate(&mut |x| engine::eval_at(expr, env, "x", x), lo, hi);
            if v.is_finite() {
                Ok((lo, hi, v))
            } else {
                Err("The integral does not converge".into())
            }
        }));
    }
    a
}

/// Every pair of visible curves that cross, listed with a dot in each curve's colour.
fn intersections_panel(ui: &mut Ui, st: &GraphState, pal: &Pal) {
    let visible = st.fns.iter().filter(|f| f.visible && f.cart().is_some()).count();
    if visible < 2 {
        return;
    }
    ui.add_space(8.0);
    widgets::caption(ui, pal, "INTERSECTIONS");
    let a = &st.analysis;
    if a.intersections.is_empty() {
        widgets::dim(ui, pal, "None in view");
        return;
    }
    for (i, j, x, y) in a.intersections.iter().take(10) {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(24.0, 18.0), Sense::hover());
            ui.painter().circle_filled(r.center() - vec2(5.0, 0.0), 4.0, pal.series[i % pal.series.len()]);
            ui.painter().circle_filled(r.center() + vec2(5.0, 0.0), 4.0, pal.series[j % pal.series.len()]);
            ui.add(egui::Label::new(RichText::new(point(*x, *y)).size(13.5)).selectable(true));
        });
    }
    if a.intersections.len() > 10 {
        widgets::dim(ui, pal, format!("+{} more", a.intersections.len() - 10));
    }
}

fn analysis_panel(ui: &mut Ui, st: &mut GraphState, pal: &Pal) {
    let Some(f) = st.fns.get(st.sel) else { return };
    let color = pal.series[st.sel % pal.series.len()];
    match &f.kind {
        Some(Ok(Kind::Cart(_))) => {}
        Some(Ok(_)) => {
            widgets::caption(ui, pal, "PARAMETER RANGE");
            ui.horizontal(|ui| {
                widgets::field(ui, pal, &mut st.t_min, "0", 84.0).on_hover_text("Where θ or t starts. Default 0.");
                widgets::dim(ui, pal, "to");
                widgets::field(ui, pal, &mut st.t_max, "one turn", 84.0)
                    .on_hover_text("Where θ or t ends. Default is one full turn: 360 in DEG, 2π in RAD.");
            });
            return;
        }
        _ => return,
    }
    ui.horizontal(|ui| {
        widgets::caption(ui, pal, "ANALYSIS");
        let (r, _) = ui.allocate_exact_size(vec2(10.0, 12.0), Sense::hover());
        ui.painter().circle_filled(r.center(), 3.5, color);
    });
    let a = &st.analysis;
    let list = |items: Vec<String>| -> String {
        if items.is_empty() {
            "none in view".into()
        } else if items.len() > 4 {
            format!("{}  +{} more", items[..4].join("   "), items.len() - 4)
        } else {
            items.join("   ")
        }
    };
    let entry = |ui: &mut Ui, label: &str, value: String| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            ui.label(RichText::new(label).color(pal.dim).size(13.0));
            ui.add(egui::Label::new(RichText::new(value).size(13.5)).selectable(true));
        });
    };
    ui.spacing_mut().item_spacing.y = 6.0;
    entry(ui, "Roots", list(a.roots.iter().map(|x| format!("x = {}", fmt::short(*x, 7))).collect()));
    let mins: Vec<String> = a.extrema.iter().filter(|e| !e.is_max).map(|e| point(e.x, e.y)).collect();
    let maxs: Vec<String> = a.extrema.iter().filter(|e| e.is_max).map(|e| point(e.x, e.y)).collect();
    let points = |ui: &mut Ui, label: &str, items: Vec<String>| {
        if items.len() <= 1 {
            entry(ui, label, list(items));
            return;
        }
        ui.label(RichText::new(label).color(pal.dim).size(13.0));
        for it in items.iter().take(5) {
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.add(egui::Label::new(RichText::new(it).size(13.5)).selectable(true));
            });
        }
        if items.len() > 5 {
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.label(RichText::new(format!("+{} more", items.len() - 5)).color(pal.dim).size(13.0));
            });
        }
    };
    points(ui, "Minimum", mins);
    points(ui, "Maximum", maxs);
    if let Some(y) = a.y_intercept {
        entry(ui, "y-intercept", point(0.0, y));
    }
    ui.add_space(8.0);
    widgets::caption(ui, pal, "INTEGRAL");
    ui.horizontal(|ui| {
        widgets::field(ui, pal, &mut st.int_a, "from", 84.0).on_hover_text("Lower limit of x. The area under the selected curve is shaded.");
        widgets::dim(ui, pal, "to");
        widgets::field(ui, pal, &mut st.int_b, "to", 84.0).on_hover_text("Upper limit of x.");
    });
    match &st.analysis.integral {
        Some(Ok((_, _, v))) => entry(ui, "∫ f(x) dx", fmt::short(*v, 10)),
        Some(Err(e)) => widgets::error(ui, pal, e.clone()),
        None => {}
    }
}

fn options_panel(ui: &mut Ui, st: &mut GraphState, pal: &Pal) {
    ui.add_space(12.0);
    widgets::caption(ui, pal, "SHOW");
    ui.spacing_mut().item_spacing.y = 4.0;
    let toggle = |ui: &mut Ui, on: &mut bool, label: &str, tip: &str| {
        let r = ui.checkbox(on, RichText::new(label).size(13.0)).on_hover_text(tip);
        let _ = r;
    };
    if let Some(f) = st.fns.get_mut(st.sel) {
        if f.cart().is_some() {
            toggle(ui, &mut f.show_deriv, "Derivative f′(x) of the selected function", "Draws the slope of the selected curve as a dashed line in the same colour.");
        }
    }
    toggle(ui, &mut st.tangent, "Tangent line while tracing", "Hover a curve to see the tangent and its slope at that point.");
    if !st.points.is_empty() {
        toggle(ui, &mut st.show_points, &format!("Statistics data ({} points)", st.points.len()), "Plots the X and Y columns from Statistics as points, so you can compare them with a function.");
    }
}

fn t_range(st: &GraphState, env: &mut Env) -> (f64, f64) {
    let lo = engine::field_value(&st.t_min, env).ok().flatten().unwrap_or(0.0);
    let hi = engine::field_value(&st.t_max, env).ok().flatten().unwrap_or(env.angle.full_turn());
    (lo, hi)
}

fn plot_ui(ui: &mut Ui, rect: Rect, st: &mut GraphState, env: &mut Env, pal: &Pal) {
    let resp = ui.interact(rect, ui.id().with("plot"), Sense::click_and_drag());
    ui.painter().rect_filled(rect, CornerRadius::same(12), pal.surface);
    // The window may still be growing on the first frames; wait for a real size.
    if !st.inited && rect.width() > 150.0 {
        let s = 20.0 / rect.width() as f64;
        st.view = View { cx: 0.0, cy: 0.0, sx: s, sy: s };
        st.inited = true;
    }
    // ---- interaction
    if resp.dragged() {
        let d = resp.drag_delta();
        st.view.cx -= d.x as f64 * st.view.sx;
        st.view.cy += d.y as f64 * st.view.sy;
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
    }
    if resp.double_clicked() {
        st.inited = false;
    }
    let hover = if resp.hovered() || resp.dragged() { ui.input(|i| i.pointer.hover_pos()) } else { None };
    if let Some(p) = hover.filter(|p| rect.contains(*p)) {
        let (scroll, pinch, mods) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta(), i.modifiers));
        let factor = (pinch as f64) * (scroll as f64 * 0.006).exp();
        if (factor - 1.0).abs() > 1e-6 {
            let (fx, fy) = if mods.shift {
                (factor, 1.0)
            } else if mods.alt {
                (1.0, factor)
            } else {
                (factor, factor)
            };
            st.view.zoom(rect, p, fx, fy);
        }
    }

    let inner = rect.shrink(1.0);
    let p = ui.painter_at(inner);
    plot::draw_grid(&p, rect, &st.view, pal);
    let (x0, x1) = st.view.x_range(rect);
    let view = st.view;

    engine::with_var(env, "x", |env| {
        // Shaded integral under the selected curve.
        // Root finding and extrema are the expensive part of a frame: only redo them when the
        // functions, the view or a slider changed.
        let key = {
            use std::fmt::Write;
            let mut k = String::with_capacity(128);
            for f in &st.fns {
                let _ = write!(k, "{}|{}|", f.text, f.visible);
            }
            for p in &st.params {
                let _ = write!(k, "{}={};", p.name, p.value);
            }
            let _ = write!(k, "{}|{:.6}|{:.6}|{}|{}|{:?}", st.sel, x0, x1, st.int_a, st.int_b, env.angle);
            k
        };
        if key != st.analysis_key {
            st.analysis = analyse(st, env, x0, x1);
            st.analysis_key = key;
        }
        if let (Some(Ok((lo, hi, _))), Some(expr)) = (&st.analysis.integral, st.fns[st.sel].cart()) {
            let color = pal.series[st.sel % pal.series.len()];
            let fill = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 44);
            let (a, b) = (lo.min(*hi).max(x0), lo.max(*hi).min(x1));
            let pa = view.to_screen(rect, a, 0.0).x;
            let pb = view.to_screen(rect, b, 0.0).x;
            let base = view.to_screen(rect, 0.0, 0.0).y;
            let mut mesh = egui::Mesh::default();
            let mut joined = false;
            let mut px = pa.ceil();
            while px <= pb {
                let x = view.cx + (px - rect.center().x) as f64 * view.sx;
                let y = engine::eval_at(expr, env, "x", x);
                if y.is_finite() {
                    let py = view.to_screen(rect, x, y).y.clamp(rect.top() - 2.0, rect.bottom() + 2.0);
                    mesh.colored_vertex(pos2(px, base.clamp(rect.top() - 2.0, rect.bottom() + 2.0)), fill);
                    mesh.colored_vertex(pos2(px, py), fill);
                    if joined {
                        let n = mesh.vertices.len() as u32;
                        mesh.add_triangle(n - 4, n - 3, n - 2);
                        mesh.add_triangle(n - 3, n - 2, n - 1);
                    }
                    joined = true;
                } else {
                    joined = false;
                }
                px += 1.0;
            }
            p.add(egui::Shape::mesh(mesh));
        }
        // Curves.
        let (t0, t1) = engine::with_var(env, "t", |env| engine::with_var(env, "θ", |env| t_range(st, env)));
        for (i, f) in st.fns.iter().enumerate() {
            if !f.visible {
                continue;
            }
            let color = pal.series[i % pal.series.len()];
            let width = if i == st.sel { 2.2 } else { 1.8 };
            if f.show_deriv {
                if let Some(e) = f.cart() {
                    plot::draw_curve_dashed(&p, rect, &view, color, 1.5, &mut |x| {
                        numeric::derivative(&mut |u| engine::eval_at(e, env, "x", u), x)
                    });
                }
            }
            match &f.kind {
                Some(Ok(Kind::Cart(e))) => {
                    plot::draw_curve(&p, rect, &view, color, width, &mut |x| engine::eval_at(e, env, "x", x));
                }
                Some(Ok(Kind::Ineq { expr: e, above, strict })) => {
                    let fill = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 40);
                    plot::fill_region(&p, inner, &view, fill, *above, &mut |x| engine::eval_at(e, env, "x", x));
                    if *strict {
                        plot::draw_curve_dashed(&p, rect, &view, color, width, &mut |x| engine::eval_at(e, env, "x", x));
                    } else {
                        plot::draw_curve(&p, rect, &view, color, width, &mut |x| engine::eval_at(e, env, "x", x));
                    }
                }
                Some(Ok(Kind::Polar(e))) => {
                    let ang = env.angle;
                    let pts: Vec<(f64, f64)> = engine::with_var(env, "θ", |env| {
                        (0..=1500)
                            .map(|k| {
                                let th = t0 + (t1 - t0) * k as f64 / 1500.0;
                                let r = engine::eval_at(e, env, "θ", th);
                                let a = ang.to_rad(th);
                                (r * a.cos(), r * a.sin())
                            })
                            .collect()
                    });
                    plot::draw_path(&p, rect, &view, color, width, &pts);
                }
                Some(Ok(Kind::Param(ex, ey))) => {
                    let pts: Vec<(f64, f64)> = engine::with_var(env, "t", |env| {
                        (0..=1500)
                            .map(|k| {
                                let t = t0 + (t1 - t0) * k as f64 / 1500.0;
                                (engine::eval_at(ex, env, "t", t), engine::eval_at(ey, env, "t", t))
                            })
                            .collect()
                    });
                    plot::draw_path(&p, rect, &view, color, width, &pts);
                }
                _ => {}
            }
        }
        // Data points from Statistics.
        if st.show_points {
            for (x, y) in &st.points {
                let pos = view.to_screen(rect, *x, *y);
                if inner.contains(pos) {
                    p.circle_filled(pos, 3.5, pal.text);
                }
            }
        }
        // Points of interest on the selected curve.
        let mut pois: Vec<(Pos2, String, Color32)> = Vec::new();
        if st.fns.get(st.sel).map(|f| f.visible && f.cart().is_some()).unwrap_or(false) {
            let color = pal.series[st.sel % pal.series.len()];
            for x in &st.analysis.roots {
                pois.push((view.to_screen(rect, *x, 0.0), format!("Root  {}", point(*x, 0.0)), color));
            }
            for e in &st.analysis.extrema {
                let name = if e.is_max { "Maximum" } else { "Minimum" };
                pois.push((view.to_screen(rect, e.x, e.y), format!("{}  {}", name, point(e.x, e.y)), color));
            }
        }
        for (_, j, x, y) in &st.analysis.intersections {
            pois.push((view.to_screen(rect, *x, *y), format!("Intersection  {}", point(*x, *y)), pal.series[j % pal.series.len()]));
        }
        for (pos, _, c) in &pois {
            if inner.contains(*pos) {
                plot::marker(&p, *pos, *c, pal);
            }
        }
        // Trace: snap to a point of interest, otherwise follow the nearest curve.
        if let Some(mp) = hover.filter(|m| inner.contains(*m) && !resp.dragged() && st.export == 0) {
            let mut tip: Option<(Pos2, String, Color32)> = pois
                .iter()
                .filter(|(pos, _, _)| pos.distance(mp) < 12.0)
                .min_by(|a, b| a.0.distance(mp).partial_cmp(&b.0.distance(mp)).unwrap())
                .cloned();
            if tip.is_none() {
                let (wx, _) = view.to_world(rect, mp);
                let mut best = 28.0f32;
                for (i, f) in st.fns.iter().enumerate() {
                    if let (true, Some(e)) = (f.visible, f.cart()) {
                        let y = engine::eval_at(e, env, "x", wx);
                        if y.is_finite() {
                            let pos = view.to_screen(rect, wx, y);
                            let d = (pos.y - mp.y).abs();
                            if d < best {
                                best = d;
                                tip = Some((pos, point(wx, y), pal.series[i % pal.series.len()]));
                            }
                        }
                    }
                }
                if let Some((pos, _, c)) = &tip {
                    p.circle_filled(*pos, 4.5, *c);
                    // Tangent line at the traced point.
                    if st.tangent {
                        if let Some(e) = st.fns.get(st.sel).and_then(|f| f.cart()) {
                            let m = numeric::derivative(&mut |u| engine::eval_at(e, env, "x", u), wx);
                            let y = engine::eval_at(e, env, "x", wx);
                            if m.is_finite() && y.is_finite() {
                                let dx = (x1 - x0) * 2.0;
                                let a = view.to_screen(rect, wx - dx, y - m * dx);
                                let b = view.to_screen(rect, wx + dx, y + m * dx);
                                p.line_segment([a, b], Stroke::new(1.2_f32, mix(*c, pal.text, 0.3)));
                                tip = Some((*pos, format!("{}   slope {}", point(wx, y), fmt::short(m, 6)), *c));
                            }
                        }
                    }
                }
            }
            if let Some((pos, text, _)) = tip {
                let galley = p.layout_no_wrap(text, theme::regular(12.5), pal.text);
                let size = galley.size() + vec2(16.0, 10.0);
                let mut at = pos + vec2(12.0, -size.y - 10.0);
                at.x = at.x.min(inner.right() - size.x - 6.0).max(inner.left() + 6.0);
                if at.y < inner.top() + 6.0 {
                    at.y = pos.y + 14.0;
                }
                let bx = Rect::from_min_size(at, size);
                p.rect_filled(bx, CornerRadius::same(7), mix(pal.surface, pal.key, 0.6));
                p.rect_stroke(bx, CornerRadius::same(7), Stroke::new(1.0_f32, pal.line), StrokeKind::Inside);
                p.galley(at + vec2(8.0, 5.0), galley, pal.text);
            }
        }
    });

    ui.painter().rect_stroke(rect, CornerRadius::same(12), Stroke::new(1.0_f32, pal.line), StrokeKind::Inside);

    // ---- PNG export: hide the controls for a frame, then take a screenshot and crop it to the plot.
    export_step(ui, rect, st);

    // ---- overlay controls
    let controls = Rect::from_min_size(pos2(rect.right() - 230.0, rect.top() + 10.0), vec2(220.0, 30.0));
    if st.export == 0 {
    ui.allocate_new_ui(UiBuilder::new().max_rect(controls).layout(Layout::right_to_left(Align::Center)), |ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if widgets::link(ui, pal, "Export PNG") {
            st.export = 1;
            st.export_rect = rect;
        }
        if widgets::link(ui, pal, "Reset") {
            st.inited = false;
        }
        if widgets::link(ui, pal, "+") {
            st.view.zoom(rect, rect.center(), 1.5, 1.5);
        }
        if widgets::link(ui, pal, "−") {
            st.view.zoom(rect, rect.center(), 1.0 / 1.5, 1.0 / 1.5);
        }
    });
    }
    if let Some((msg, t)) = &st.export_msg {
        let now = ui.input(|i| i.time);
        if now - t < 4.0 {
            ui.painter().text(
                pos2(rect.center().x, rect.bottom() - 26.0),
                Align2::CENTER_CENTER,
                msg,
                theme::regular(12.5),
                pal.accent,
            );
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
        }
    }
    if st.fns.iter().all(|f| f.kind.is_none()) {
        ui.painter().text(
            pos2(rect.center().x, rect.bottom() - 26.0),
            Align2::CENTER_CENTER,
            "Enter a function on the left.  Drag to pan, scroll to zoom, double-click to reset.",
            theme::regular(12.5),
            pal.dim,
        );
    }
}

pub fn graph_ui(ui: &mut Ui, st: &mut GraphState, env: &mut Env, pal: &Pal) {
    let (left, right) = split(ui);
    // The plot runs first so the analysis shown on the left is from this frame.
    for f in st.fns.iter_mut() {
        f.refresh();
    }
    st.sel = st.sel.min(st.fns.len() - 1);
    refresh_params(st, env);
    for p in &st.params {
        env.set_real(&p.name, p.value);
    }
    plot_ui(ui, right, st, env, pal);
    ui.allocate_new_ui(UiBuilder::new().max_rect(left), |ui| {
        ScrollArea::vertical().id_salt("graph_side").auto_shrink([false, false]).show(ui, |ui| {
            ui.set_width(left.width() - 8.0);
            function_list(ui, st, pal);
            params_panel(ui, st, env, pal);
            ui.add_space(14.0);
            analysis_panel(ui, st, pal);
            intersections_panel(ui, st, pal);
            options_panel(ui, st, pal);
        });
    });
}

pub fn table_ui(ui: &mut Ui, st: &mut GraphState, env: &mut Env, pal: &Pal) {
    for p in &st.params {
        env.set_real(&p.name, p.value);
    }
    let (left, right) = split(ui);
    let cols: Vec<usize> = (0..st.fns.len()).filter(|i| st.fns[*i].cart().is_some()).collect();
    let range = (|| -> Result<(f64, f64, usize), String> {
        let a = engine::field_value(&st.tb_start, env)?.ok_or("Enter a start value")?;
        let b = engine::field_value(&st.tb_end, env)?.ok_or("Enter an end value")?;
        let s = engine::field_value(&st.tb_step, env)?.ok_or("Enter a step")?;
        if s == 0.0 || (b - a) / s < 0.0 {
            return Err("The step must move from start towards end".into());
        }
        let n = ((b - a) / s + 1e-9).floor() as usize + 1;
        if n > 5000 {
            return Err("That is more than 5000 rows. Use a larger step".into());
        }
        Ok((a, s, n))
    })();
    ui.allocate_new_ui(UiBuilder::new().max_rect(left), |ui| {
        ScrollArea::vertical().id_salt("table_side").auto_shrink([false, false]).show(ui, |ui| {
            ui.set_width(left.width() - 8.0);
            function_list(ui, st, pal);
            ui.add_space(14.0);
            widgets::caption(ui, pal, "RANGE");
            for (label, text) in
                [("Start", &mut st.tb_start), ("End", &mut st.tb_end), ("Step", &mut st.tb_step)]
            {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(44.0, 30.0), Sense::hover());
                    ui.painter().text(r.left_center(), Align2::LEFT_CENTER, label, theme::regular(13.0), pal.dim);
                    widgets::field(ui, pal, text, "", 120.0).on_hover_text(help::NUMBER);
                });
            }
            // Copy as tab-separated text for a spreadsheet.
            if let (Ok((a, s, n)), false) = (&range, cols.is_empty()) {
                let (a, s, n) = (*a, *s, *n);
                ui.add_space(8.0);
                let mut head: Vec<String> = vec!["x".into()];
                head.extend(cols.iter().map(|c| format!("y{}", c + 1)));
                let head_refs: Vec<&str> = head.iter().map(String::as_str).collect();
                let fns = &st.fns;
                engine::with_var(env, "x", |env| {
                    let env = std::cell::RefCell::new(env);
                    widgets::copy_table(ui, pal, &head_refs, || {
                        let mut env = env.borrow_mut();
                        (0..n)
                            .map(|i| {
                                let x = a + s * i as f64;
                                let mut row = vec![fmt::short(x, 10)];
                                row.extend(cols.iter().map(|c| fmt::short(engine::eval_at(fns[*c].cart().unwrap(), &mut env, "x", x), 10)));
                                row
                            })
                            .collect()
                    });
                });
            }
        });
    });

    ui.painter().rect_filled(right, CornerRadius::same(12), pal.surface);
    ui.painter().rect_stroke(right, CornerRadius::same(12), Stroke::new(1.0_f32, pal.line), StrokeKind::Inside);
    let body = right.shrink2(vec2(18.0, 12.0));
    ui.allocate_new_ui(UiBuilder::new().max_rect(body), |ui| {
        let (a, s, n) = match range {
            Ok(r) => r,
            Err(e) => {
                widgets::error(ui, pal, e);
                return;
            }
        };
        if cols.is_empty() {
            widgets::dim(ui, pal, "Enter a function of x on the left to tabulate it.");
            return;
        }
        let col_w = ((body.width() - 8.0) / (cols.len() + 1) as f32).clamp(90.0, 220.0);
        let row_h = 28.0;
        // Header
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let (r, _) = ui.allocate_exact_size(vec2(col_w, row_h), Sense::hover());
            ui.painter().text(r.right_center() - vec2(14.0, 0.0), Align2::RIGHT_CENTER, "x", theme::medium(13.0), pal.dim);
            for c in &cols {
                let (r, _) = ui.allocate_exact_size(vec2(col_w, row_h), Sense::hover());
                let color = pal.series[*c % pal.series.len()];
                let end = ui.painter().text(
                    r.right_center() - vec2(14.0, 0.0),
                    Align2::RIGHT_CENTER,
                    format!("y{}", c + 1),
                    theme::medium(13.0),
                    pal.dim,
                );
                ui.painter().circle_filled(pos2(end.left() - 9.0, r.center().y), 3.5, color);
            }
        });
        let line_y = ui.cursor().top() - 4.0;
        ui.painter().hline(body.x_range(), line_y, Stroke::new(1.0_f32, pal.line));
        engine::with_var(env, "x", |env| {
            ScrollArea::vertical().id_salt("table_rows").auto_shrink([false, false]).show_rows(ui, row_h, n, |ui, rows| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                for i in rows {
                    let x = a + s * i as f64;
                    let x = if (x / s).abs() < 1e-9 { 0.0 } else { x };
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(col_w, row_h), Sense::hover());
                        ui.painter().text(
                            r.right_center() - vec2(14.0, 0.0),
                            Align2::RIGHT_CENTER,
                            fmt::short(x, 10),
                            theme::regular(14.0),
                            pal.dim,
                        );
                        for c in &cols {
                            let y = engine::eval_at(st.fns[*c].cart().unwrap(), env, "x", x);
                            let (r, _) = ui.allocate_exact_size(vec2(col_w, row_h), Sense::hover());
                            ui.painter().text(
                                r.right_center() - vec2(14.0, 0.0),
                                Align2::RIGHT_CENTER,
                                fmt::short(y, 10),
                                theme::regular(14.0),
                                pal.text,
                            );
                        }
                    });
                }
            });
        });
    });
}

fn export_step(ui: &mut Ui, rect: Rect, st: &mut GraphState) {
    match st.export {
        1 => {
            st.export = 2;
            st.export_rect = rect;
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            ui.ctx().request_repaint();
        }
        2 => {
            let shot = ui.input(|i| {
                i.raw.events.iter().find_map(|e| match e {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });
            if let Some(image) = shot {
                st.export = 0;
                let ppp = ui.ctx().pixels_per_point();
                let img = image.region(&st.export_rect, Some(ppp));
                let now = ui.input(|i| i.time);
                ui.ctx().copy_image(img.clone());
                st.export_msg = Some((
                    match save_png(&img) {
                        Ok(path) => format!("Saved {} and copied it to the clipboard", path),
                        Err(e) => format!("Copied to the clipboard. Could not save the file: {}", e),
                    },
                    now,
                ));
            } else {
                ui.ctx().request_repaint();
            }
        }
        _ => {}
    }
}

/// Save to the Downloads folder (or the home folder) as Scientific-Calculator-graph-<date>-<time>.png.
fn save_png(img: &egui::ColorImage) -> Result<String, String> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).ok_or("no home folder")?;
    let home = std::path::PathBuf::from(home);
    let dir = if home.join("Downloads").is_dir() { home.join("Downloads") } else { home };
    // Local time, so the name matches the clock on the user's computer.
    let name = format!("Scientific-Calculator-graph-{}.png", chrono::Local::now().format("%Y%m%d-%H%M%S"));
    let path = dir.join(&name);
    let bytes: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
    image::save_buffer(&path, &bytes, img.size[0] as u32, img.size[1] as u32, image::ExtendedColorType::Rgba8)
        .map_err(|e| e.to_string())?;
    Ok(if dir.ends_with("Downloads") { format!("Downloads/{}", name) } else { name })
}
