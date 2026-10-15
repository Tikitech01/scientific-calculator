//! Matrix mode: six editable matrices and an expression line that works on them.

use crate::engine::{self, Env, Value};
use crate::fmt::{self, Fmt};
use crate::help;
use crate::matrix::Matrix;
use crate::theme::{self, Pal};
use crate::widgets;
use eframe::egui::{self, vec2, Align, Align2, Id, Key, RichText, ScrollArea, Sense, Ui};

const NAMES: [&str; 6] = ["A", "B", "C", "D", "E", "F"];
const MAX: usize = 6;

pub struct MatState {
    cells: Vec<Vec<Vec<String>>>,
    dims: Vec<(usize, usize)>,
    pub sel: usize,
    pub expr: String,
    pub result: Option<Result<Value, String>>,
    cell_error: Option<String>,
}

impl Default for MatState {
    fn default() -> Self {
        MatState {
            cells: vec![vec![vec![String::new(); MAX]; MAX]; 6],
            dims: vec![(2, 2); 6],
            sel: 0,
            expr: String::new(),
            result: None,
            cell_error: None,
        }
    }
}

impl MatState {
    /// The selected matrix as typed, one row per line, for a CSV file.
    pub fn csv_rows(&self) -> Vec<Vec<String>> {
        let (r, c) = self.dims[self.sel];
        (0..r).map(|i| (0..c).map(|j| self.cells[self.sel][i][j].clone()).collect()).collect()
    }

    /// Name of the selected matrix, MatA … MatF.
    pub fn name(&self) -> String {
        format!("Mat{}", NAMES[self.sel])
    }

    fn build(&self, k: usize, env: &mut Env) -> Result<Matrix, String> {
        let (r, c) = self.dims[k];
        let mut m = Matrix::zeros(r, c);
        for i in 0..r {
            for j in 0..c {
                if let Some(v) = engine::field_value(&self.cells[k][i][j], env)? {
                    m.set(i, j, v);
                }
            }
        }
        Ok(m)
    }

    /// Write the edited matrices into the environment as MatA … MatF.
    pub fn push(&mut self, env: &mut Env) {
        self.cell_error = None;
        for k in 0..6 {
            match self.build(k, env) {
                Ok(m) => {
                    env.vars.insert(format!("Mat{}", NAMES[k]), Value::Mat(m));
                }
                Err(e) => {
                    if k == self.sel {
                        self.cell_error = Some(e);
                    }
                }
            }
        }
    }

    fn load(&mut self, k: usize, m: &Matrix) {
        if m.r > MAX || m.c > MAX {
            return;
        }
        self.dims[k] = (m.r, m.c);
        for i in 0..MAX {
            for j in 0..MAX {
                self.cells[k][i][j] =
                    if i < m.r && j < m.c { fmt::plain(&Value::real(m.get(i, j))) } else { String::new() };
            }
        }
    }

    /// Pick up matrices stored from elsewhere (for example "… → MatA" in Calculate).
    pub fn pull(&mut self, env: &mut Env) {
        for k in 0..6 {
            let stored = match env.vars.get(&format!("Mat{}", NAMES[k])) {
                Some(Value::Mat(m)) => m.clone(),
                _ => continue,
            };
            if self.build(k, env).ok().as_ref() != Some(&stored) {
                self.load(k, &stored);
            }
        }
    }
}

fn stepper(ui: &mut Ui, pal: &Pal, label: &str, v: &mut usize) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        widgets::dim(ui, pal, label);
        ui.add_space(4.0);
        if widgets::glyph(ui, pal, "−", 24.0).clicked() && *v > 1 {
            *v -= 1;
        }
        let (r, _) = ui.allocate_exact_size(vec2(18.0, 24.0), Sense::hover());
        ui.painter().text(r.center(), Align2::CENTER_CENTER, v.to_string(), theme::medium(14.0), pal.text);
        if widgets::glyph(ui, pal, "+", 24.0).clicked() && *v < MAX {
            *v += 1;
        }
    });
}

pub fn matrix_view(ui: &mut Ui, pal: &Pal, m: &Matrix) {
    let texts: Vec<String> = m.d.iter().map(|v| fmt::short(*v, 8)).collect();
    let w = texts
        .iter()
        .map(|t| widgets::text_width(ui, t, theme::regular(15.0)))
        .fold(30.0f32, f32::max)
        + 22.0;
    let h = 30.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w * m.c as f32 + 16.0, h * m.r as f32 + 8.0), Sense::hover());
    let p = ui.painter();
    // Brackets
    let s = egui::Stroke::new(1.5_f32, pal.dim);
    for (x, dir) in [(rect.left() + 2.0, 1.0), (rect.right() - 2.0, -1.0)] {
        p.line_segment([egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())], s);
        p.line_segment([egui::pos2(x, rect.top()), egui::pos2(x + 6.0 * dir, rect.top())], s);
        p.line_segment([egui::pos2(x, rect.bottom()), egui::pos2(x + 6.0 * dir, rect.bottom())], s);
    }
    for i in 0..m.r {
        for j in 0..m.c {
            let c = egui::pos2(rect.left() + 8.0 + w * (j as f32 + 0.5), rect.top() + 4.0 + h * (i as f32 + 0.5));
            p.text(c, Align2::CENTER_CENTER, &texts[i * m.c + j], theme::regular(15.0), pal.text);
        }
    }
}

pub fn ui(ui: &mut Ui, st: &mut MatState, env: &mut Env, f: Fmt, pal: &Pal) {
    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.horizontal(|ui| {
            widgets::segmented(ui, pal, &mut st.sel, &NAMES);
            ui.add_space(12.0);
            let (mut r, mut c) = st.dims[st.sel];
            stepper(ui, pal, "Rows", &mut r);
            ui.add_space(6.0);
            stepper(ui, pal, "Columns", &mut c);
            st.dims[st.sel] = (r, c);
        });
        ui.add_space(8.0);
        let (rows, cols) = st.dims[st.sel];
        let k = st.sel;
        for i in 0..rows {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for j in 0..cols {
                    widgets::field_with(ui, pal, &mut st.cells[k][i][j], "0", 62.0, Align::Center, None)
                        .on_hover_text("A number or a calculation, e.g. 1/3. Empty counts as 0. Tab moves to the next cell.");
                }
            });
        }
        st.push(env);
        if let Some(e) = &st.cell_error {
            widgets::error(ui, pal, e.clone());
        }

        ui.add_space(18.0);
        widgets::caption(ui, pal, "CALCULATE");
        let name = format!("Mat{}", NAMES[k]);
        let mut run = false;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            for (label, expr) in [
                ("Determinant", format!("det({})", name)),
                ("Inverse", format!("{}⁻¹", name)),
                ("Transpose", format!("trn({})", name)),
                ("Row-reduce", format!("rref({})", name)),
                ("Rank", format!("rank({})", name)),
                ("Trace", format!("trace({})", name)),
                ("Square", format!("{}²", name)),
                ("Eigenvalues", format!("eig({})", name)),
                ("Eigenvectors", format!("eigvec({})", name)),
            ] {
                if widgets::link(ui, pal, label) {
                    st.expr = expr;
                    run = true;
                }
            }
        });
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            let id = Id::new("matrix_expr");
            let r = widgets::field_with(
                ui,
                pal,
                &mut st.expr,
                "MatA*MatB + 2MatC",
                (ui.available_width() - 110.0).min(460.0),
                Align::LEFT,
                Some(id),
            )
            .on_hover_text(help::MATRIX_EXPR);
            if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                run = true;
            }
            if widgets::link(ui, pal, "Calculate") {
                run = true;
            }
        });
        if run && !st.expr.trim().is_empty() {
            st.result = Some(engine::evaluate(&st.expr, env));
        }
        ui.add_space(10.0);
        let mut store: Option<usize> = None;
        match &st.result {
            None => widgets::dim(
                ui,
                pal,
                "Use MatA to MatF in any expression. A result matrix is kept as MatAns.",
            ),
            Some(Err(e)) => widgets::error(ui, pal, e.clone()),
            Some(Ok(Value::Mat(m))) => {
                matrix_view(ui, pal, m);
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    widgets::dim(ui, pal, "Store in");
                    for (i, n) in NAMES.iter().enumerate() {
                        if widgets::link(ui, pal, n) {
                            store = Some(i);
                        }
                    }
                });
            }
            Some(Ok(v)) => {
                ui.add(
                    egui::Label::new(RichText::new(fmt::value(v, f)).font(theme::light(30.0))).selectable(true),
                );
            }
        }
        if let (Some(i), Some(Ok(Value::Mat(m)))) = (store, st.result.clone()) {
            st.load(i, &m);
            st.sel = i;
            st.push(env);
        }
    });
}
