//! Statistics mode: a list editor with one-variable statistics, regression and distributions.

use crate::engine::{self, Env, Value};
use crate::fmt::{self, Fmt};
use crate::help;
use crate::plot::{self, View};
use crate::stats::{self, Model};
use crate::theme::{self, Pal};
use crate::widgets;
use eframe::egui::{
    pos2, vec2, Align, Align2, CornerRadius, Rect, ScrollArea, Sense, Stroke, StrokeKind, Ui, UiBuilder,
};

const COLS: [&str; 3] = ["X", "Y", "Freq"];

pub struct StatsState {
    rows: Vec<[String; 3]>,
    pub sub: usize,
    pub column: usize,
    pub model: usize,
    pub predict: String,
    pub dist: usize,
    /// Inputs for each distribution: Normal, Binomial, Poisson, Student t, χ², F.
    pub params: [[String; 5]; 6],
    pub tests: crate::ui_tests::TestsState,
}

impl Default for StatsState {
    fn default() -> Self {
        StatsState {
            rows: vec![Default::default(); 8],
            sub: 0,
            column: 0,
            model: 0,
            predict: String::new(),
            dist: 0,
            params: Default::default(),
            tests: Default::default(),
        }
    }
}

impl StatsState {
    fn column(&self, c: usize, env: &mut Env) -> Vec<Option<f64>> {
        self.rows.iter().map(|r| engine::field_value(&r[c], env).ok().flatten()).collect()
    }

    fn list(&self, c: usize, env: &mut Env) -> Vec<f64> {
        self.column(c, env).into_iter().flatten().collect()
    }

    /// Publish the columns as List1, List2, List3.
    pub fn push(&self, env: &mut Env) {
        for c in 0..3 {
            let l = self.list(c, env);
            env.vars.insert(format!("List{}", c + 1), Value::List(l));
        }
    }

    /// Pick up lists stored from Calculate ("{1,2,3} → List1").
    pub fn pull(&mut self, env: &mut Env) {
        for c in 0..3 {
            let stored = match env.vars.get(&format!("List{}", c + 1)) {
                Some(Value::List(l)) => l.clone(),
                _ => continue,
            };
            if self.list(c, env) != stored {
                while self.rows.len() < stored.len() + 1 {
                    self.rows.push(Default::default());
                }
                for (i, row) in self.rows.iter_mut().enumerate() {
                    row[c] = stored.get(i).map(|v| fmt::plain(&Value::real(*v))).unwrap_or_default();
                }
            }
        }
    }

    pub fn load_demo(&mut self) {
        let data = [(1.0, 2.1), (2.0, 3.9), (3.0, 6.2), (4.0, 7.8), (5.0, 10.1), (6.0, 12.2), (7.0, 13.8)];
        self.rows = data.iter().map(|(x, y)| [x.to_string(), y.to_string(), String::new()]).collect();
        self.rows.push(Default::default());
    }
}

pub fn ui(ui: &mut Ui, st: &mut StatsState, env: &mut Env, f: Fmt, pal: &Pal) {
    let avail = ui.available_rect_before_wrap();
    ui.allocate_rect(avail, Sense::hover());
    let left = Rect::from_min_max(avail.min, pos2(avail.left() + 290.0, avail.bottom()));
    let right = Rect::from_min_max(pos2(left.right() + 22.0, avail.top()), avail.max);

    // ---- list editor
    ui.allocate_new_ui(UiBuilder::new().max_rect(left), |ui| {
        let w = 78.0;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.add_space(26.0);
            for (i, name) in COLS.iter().enumerate() {
                let (r, _) = ui.allocate_exact_size(vec2(w, 20.0), Sense::hover());
                ui.painter().text(r.center(), Align2::CENTER_CENTER, *name, theme::medium(12.0), pal.dim);
                let _ = i;
            }
        });
        ScrollArea::vertical().id_salt("stat_rows").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 5.0);
            for (i, row) in st.rows.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(20.0, 30.0), Sense::hover());
                    ui.painter().text(
                        r.right_center(),
                        Align2::RIGHT_CENTER,
                        (i + 1).to_string(),
                        theme::regular(11.5),
                        pal.faint,
                    );
                    for c in 0..3 {
                        widgets::field_with(ui, pal, &mut row[c], "", w - 20.0, Align::Center, None)
                            .on_hover_text(if c == 2 { help::FREQ } else { help::DATA });
                    }
                });
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(26.0);
                if widgets::link(ui, pal, "Clear all") {
                    st.rows = vec![Default::default(); 8];
                }
            });
        });
    });
    // Always keep an empty row at the end.
    if st.rows.last().map(|r| r.iter().any(|c| !c.trim().is_empty())).unwrap_or(true) {
        st.rows.push(Default::default());
    }
    st.push(env);

    // ---- results
    ui.allocate_new_ui(UiBuilder::new().max_rect(right), |ui| {
        widgets::segmented(ui, pal, &mut st.sub, &["One variable", "Regression", "Distributions", "Tests"]);
        ui.add_space(12.0);
        match st.sub {
            0 => one_var(ui, st, env, f, pal),
            1 => regression(ui, st, env, f, pal),
            2 => distributions(ui, st, env, f, pal),
            _ => {
                let x = st.list(0, env);
                let y = st.list(1, env);
                crate::ui_tests::ui(ui, &mut st.tests, &x, &y, env, f, pal);
            }
        }
    });
}

fn one_var(ui: &mut Ui, st: &mut StatsState, env: &mut Env, f: Fmt, pal: &Pal) {
    ui.horizontal(|ui| {
        widgets::dim(ui, pal, "Data from");
        widgets::segmented(ui, pal, &mut st.column, &["X", "Y"]);
    });
    ui.add_space(10.0);
    let data = st.column(st.column.min(1), env);
    let freq = st.column(2, env);
    let use_freq = freq.iter().any(|v| v.is_some());
    let mut xs = Vec::new();
    let mut ws = Vec::new();
    for (x, w) in data.iter().zip(&freq) {
        if let Some(x) = x {
            xs.push(*x);
            ws.push(w.unwrap_or(1.0));
        }
    }
    if xs.is_empty() {
        let col = st.column.min(1);
        let other = 1 - col;
        if st.column(other, env).iter().any(|v| v.is_some()) {
            let names = ["X", "Y"];
            ui.horizontal(|ui| {
                widgets::dim(ui, pal, format!("Column {} is empty, but {} has values.", names[col], names[other]));
                if widgets::link(ui, pal, &format!("Use {}", names[other])) {
                    st.column = other;
                }
            });
        } else {
            widgets::dim(ui, pal, "Enter values in the list on the left. Freq is optional and counts each value that many times.");
        }
        return;
    }
    let s = match stats::one_var(&xs, if use_freq { Some(&ws) } else { None }) {
        Ok(s) => s,
        Err(e) => {
            widgets::error(ui, pal, e);
            return;
        }
    };
    let v = |x: f64| fmt::real(x, f);
    ScrollArea::vertical().id_salt("onevar").auto_shrink([false, false]).show(ui, |ui| {
        ui.set_max_width(420.0);
        widgets::card(ui, pal, |ui| {
            ui.spacing_mut().item_spacing.y = 3.0;
            widgets::row(ui, pal, "Count  n", &v(s.n));
            widgets::row(ui, pal, "Mean", &v(s.mean));
            widgets::row(ui, pal, "Sum  Σx", &v(s.sum));
            widgets::row(ui, pal, "Sum of squares  Σx²", &v(s.sum_sq));
            widgets::row(ui, pal, "Standard deviation, population  σ", &v(s.pop_sd));
            widgets::row(
                ui,
                pal,
                "Standard deviation, sample  s",
                &if s.sample_sd.is_nan() { "–".to_string() } else { v(s.sample_sd) },
            );
        });
        ui.add_space(4.0);
        widgets::card(ui, pal, |ui| {
            ui.spacing_mut().item_spacing.y = 3.0;
            widgets::row(ui, pal, "Minimum", &v(s.min));
            widgets::row(ui, pal, "First quartile  Q1", &v(s.q1));
            widgets::row(ui, pal, "Median", &v(s.median));
            widgets::row(ui, pal, "Third quartile  Q3", &v(s.q3));
            widgets::row(ui, pal, "Maximum", &v(s.max));
            let mode = if s.mode.is_empty() {
                "none".to_string()
            } else {
                s.mode.iter().take(5).map(|m| v(*m)).collect::<Vec<_>>().join(", ")
            };
            widgets::row(ui, pal, "Mode", &mode);
        });
    });
}

fn regression(ui: &mut Ui, st: &mut StatsState, env: &mut Env, f: Fmt, pal: &Pal) {
    let names: Vec<&str> = Model::ALL.iter().map(|m| m.name()).collect();
    widgets::segmented(ui, pal, &mut st.model, &names);
    ui.add_space(10.0);
    let xs_all = st.column(0, env);
    let ys_all = st.column(1, env);
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for (x, y) in xs_all.iter().zip(&ys_all) {
        if let (Some(x), Some(y)) = (x, y) {
            xs.push(*x);
            ys.push(*y);
        }
    }
    if xs.is_empty() {
        widgets::dim(ui, pal, "Enter paired values in the X and Y columns.");
        return;
    }
    let model = Model::ALL[st.model];
    let fit = stats::regress(model, &xs, &ys);
    let avail = ui.available_rect_before_wrap();
    let info_w = 250.0;
    let info = Rect::from_min_max(avail.min, pos2(avail.left() + info_w, avail.bottom()));
    let chart = Rect::from_min_max(pos2(info.right() + 16.0, avail.top()), avail.max);
    ui.allocate_rect(avail, Sense::hover());

    ui.allocate_new_ui(UiBuilder::new().max_rect(info), |ui| {
        widgets::dim(ui, pal, model.formula());
        ui.add_space(2.0);
        match &fit {
            Err(e) => widgets::error(ui, pal, e.clone()),
            Ok(r) => {
                widgets::card(ui, pal, |ui| {
                    ui.spacing_mut().item_spacing.y = 3.0;
                    for (name, c) in ["a", "b", "c", "d"].iter().zip(&r.coef) {
                        widgets::row(ui, pal, name, &fmt::real(*c, f));
                    }
                    if let Some(rv) = r.r {
                        widgets::row(ui, pal, "r", &fmt::real(rv, f));
                    }
                    widgets::row(ui, pal, "r²", &fmt::real(r.r2, f));
                });
                ui.add_space(8.0);
                widgets::caption(ui, pal, "PREDICT");
                ui.horizontal(|ui| {
                    widgets::dim(ui, pal, "x =");
                    widgets::field(ui, pal, &mut st.predict, "", 90.0)
                        .on_hover_text("Enter an x value to get the y the fitted curve predicts.");
                });
                match engine::field_value(&st.predict, env) {
                    Ok(Some(x)) => widgets::row(ui, pal, "y =", &fmt::real(r.predict(x), f)),
                    Ok(None) => {}
                    Err(e) => widgets::error(ui, pal, e),
                }
            }
        }
    });

    // ---- scatter chart
    if chart.width() < 120.0 || chart.height() < 120.0 {
        return;
    }
    ui.painter().rect_filled(chart, CornerRadius::same(12), pal.surface);
    let p = ui.painter_at(chart.shrink(1.0));
    let (mut x0, mut x1) = xs.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, v| (a.0.min(*v), a.1.max(*v)));
    let (mut y0, mut y1) = ys.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, v| (a.0.min(*v), a.1.max(*v)));
    if x1 - x0 < 1e-12 {
        x0 -= 1.0;
        x1 += 1.0;
    }
    if y1 - y0 < 1e-12 {
        y0 -= 1.0;
        y1 += 1.0;
    }
    let (px, py) = ((x1 - x0) * 0.18, (y1 - y0) * 0.22);
    let mut view = View::default();
    view.fit(chart, x0 - px, x1 + px, y0 - py, y1 + py);
    plot::draw_grid(&p, chart, &view, pal);
    if let Ok(r) = &fit {
        plot::draw_curve(&p, chart, &view, pal.series[1], 2.0, &mut |x| r.predict(x));
    }
    for (x, y) in xs.iter().zip(&ys) {
        p.circle(view.to_screen(chart, *x, *y), 4.0, pal.series[0], Stroke::new(1.5_f32, pal.surface));
    }
    ui.painter().rect_stroke(chart, CornerRadius::same(12), Stroke::new(1.0_f32, pal.line), StrokeKind::Inside);
}

fn distributions(ui: &mut Ui, st: &mut StatsState, env: &mut Env, f: Fmt, pal: &Pal) {
    widgets::segmented(ui, pal, &mut st.dist, &["Normal", "Binomial", "Poisson", "Student t", "χ²", "F"]);
    ui.add_space(12.0);
    let d = st.dist.min(5);
    let labels: &[(&str, &str)] = match d {
        0 => &[("Mean  μ", "0"), ("Std deviation  σ", "1"), ("Lower", "−∞"), ("Upper", "+∞"), ("Inverse: area left", "e.g. 0.975")],
        1 => &[("Trials  n", ""), ("Probability  p", ""), ("Successes  k", "")],
        2 => &[("Mean  λ", ""), ("Events  k", "")],
        3 => &[("Degrees of freedom  ν", ""), ("Lower", "−∞"), ("Upper", "+∞"), ("Inverse: area left", "e.g. 0.975")],
        4 => &[("Degrees of freedom  k", ""), ("Lower", "0"), ("Upper", "+∞"), ("Inverse: area left", "e.g. 0.95")],
        _ => &[("df₁  numerator", ""), ("df₂  denominator", ""), ("Lower", "0"), ("Upper", "+∞"), ("Inverse: area left", "e.g. 0.95")],
    };
    for (i, (label, hint)) in labels.iter().enumerate() {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(150.0, 30.0), Sense::hover());
            ui.painter().text(r.left_center(), Align2::LEFT_CENTER, *label, theme::regular(13.5), pal.dim);
            let tip = if label.starts_with("Inverse") {
                "Optional. A probability between 0 and 1: the value x with that much area to its left (a critical value)."
            } else {
                help::NUMBER
            };
            widgets::field(ui, pal, &mut st.params[d][i], hint, 110.0).on_hover_text(tip);
        });
    }
    ui.add_space(12.0);
    let mut get = |i: usize| engine::field_value(&st.params[d][i], env);
    let v = |x: f64| fmt::real(x, f);
    // Rows for a continuous distribution: interval probability, densities and the inverse.
    let continuous = |cdf: &dyn Fn(f64) -> f64,
                      pdf: &dyn Fn(f64) -> f64,
                      inv: &dyn Fn(f64) -> f64,
                      lo: Option<f64>,
                      hi: Option<f64>,
                      p: Option<f64>|
     -> Result<Vec<(String, String)>, String> {
        let mut rows = Vec::new();
        if lo.is_some() || hi.is_some() {
            let cl = lo.map(cdf).unwrap_or(0.0);
            let ch = hi.map(cdf).unwrap_or(1.0);
            rows.push(("P(lower ≤ X ≤ upper)".to_string(), v((ch - cl).max(0.0))));
            if let Some(x) = lo {
                rows.push(("Density at lower".into(), v(pdf(x))));
            }
            if let Some(x) = hi {
                rows.push(("Density at upper".into(), v(pdf(x))));
            }
        }
        if let Some(p) = p {
            if !(p > 0.0 && p < 1.0) {
                return Err("The area must be between 0 and 1".into());
            }
            rows.push((format!("x with area {} to its left", fmt::short(p, 6)), v(inv(p))));
        }
        Ok(rows)
    };
    let out: Result<Vec<(String, String)>, String> = (|| {
        Ok(match d {
            0 => {
                let mu = get(0)?.unwrap_or(0.0);
                let sigma = get(1)?.unwrap_or(1.0);
                if sigma <= 0.0 {
                    return Err("σ must be positive".into());
                }
                let (lo, hi, p) = (get(2)?, get(3)?, get(4)?);
                let mut rows = continuous(
                    &|x| stats::norm_cdf(x, mu, sigma),
                    &|x| stats::norm_pdf(x, mu, sigma),
                    &|p| mu + sigma * stats::inv_norm(p),
                    lo,
                    hi,
                    p,
                )?;
                if let Some(x) = lo {
                    rows.push(("z-score of lower".into(), v((x - mu) / sigma)));
                }
                if let Some(x) = hi {
                    rows.push(("z-score of upper".into(), v((x - mu) / sigma)));
                }
                rows
            }
            1 => {
                let (Some(n), Some(p), Some(k)) = (get(0)?, get(1)?, get(2)?) else {
                    return Ok(vec![]);
                };
                if n < 0.0 || n.fract() != 0.0 || !(0.0..=1.0).contains(&p) {
                    return Err("n must be a whole number ≥ 0 and p between 0 and 1".into());
                }
                let cdf = stats::binom_cdf(k, n, p);
                vec![
                    ("P(X = k)".into(), v(stats::binom_pdf(k, n, p))),
                    ("P(X ≤ k)".into(), v(cdf)),
                    ("P(X > k)".into(), v((1.0 - cdf).max(0.0))),
                    ("Mean  n·p".into(), v(n * p)),
                    ("Std deviation".into(), v((n * p * (1.0 - p)).sqrt())),
                ]
            }
            2 => {
                let (Some(l), Some(k)) = (get(0)?, get(1)?) else {
                    return Ok(vec![]);
                };
                if l <= 0.0 {
                    return Err("λ must be positive".into());
                }
                let cdf = stats::poisson_cdf(k, l);
                vec![
                    ("P(X = k)".into(), v(stats::poisson_pdf(k, l))),
                    ("P(X ≤ k)".into(), v(cdf)),
                    ("P(X > k)".into(), v((1.0 - cdf).max(0.0))),
                ]
            }
            3 => {
                let Some(df) = get(0)? else { return Ok(vec![]) };
                if df <= 0.0 {
                    return Err("ν must be positive".into());
                }
                let mut rows = continuous(
                    &|x| crate::hypothesis::t_cdf(x, df),
                    &|x| crate::hypothesis::t_pdf(x, df),
                    &|p| crate::hypothesis::inv_t(p, df),
                    get(1)?,
                    get(2)?,
                    get(3)?,
                )?;
                if !rows.is_empty() {
                    rows.push(("Mean".into(), if df > 1.0 { v(0.0) } else { "undefined".into() }));
                    rows.push(("Variance".into(), if df > 2.0 { v(df / (df - 2.0)) } else { "∞".into() }));
                }
                rows
            }
            4 => {
                let Some(k) = get(0)? else { return Ok(vec![]) };
                if k <= 0.0 {
                    return Err("k must be positive".into());
                }
                let mut rows = continuous(
                    &|x| crate::hypothesis::chi2_cdf(x, k),
                    &|x| crate::hypothesis::chi2_pdf(x, k),
                    &|p| crate::hypothesis::inv_chi2(p, k),
                    get(1)?,
                    get(2)?,
                    get(3)?,
                )?;
                if !rows.is_empty() {
                    rows.push(("Mean".into(), v(k)));
                    rows.push(("Variance".into(), v(2.0 * k)));
                }
                rows
            }
            _ => {
                let (Some(d1), Some(d2)) = (get(0)?, get(1)?) else { return Ok(vec![]) };
                if d1 <= 0.0 || d2 <= 0.0 {
                    return Err("Both degrees of freedom must be positive".into());
                }
                let mut rows = continuous(
                    &|x| crate::hypothesis::f_cdf(x, d1, d2),
                    &|x| crate::hypothesis::f_pdf(x, d1, d2),
                    &|p| crate::hypothesis::inv_f(p, d1, d2),
                    get(2)?,
                    get(3)?,
                    get(4)?,
                )?;
                if !rows.is_empty() && d2 > 2.0 {
                    rows.push(("Mean".into(), v(d2 / (d2 - 2.0))));
                }
                rows
            }
        })
    })();
    match out {
        Err(e) => widgets::error(ui, pal, e),
        Ok(rows) if rows.is_empty() => widgets::dim(
            ui,
            pal,
            match d {
                1 | 2 => "Fill in every field to see the probabilities.",
                0 => "Enter a lower bound, an upper bound, or both (a blank bound means unbounded), or an area for the inverse.",
                _ => "Enter the degrees of freedom, then a lower and/or upper bound, or an area to find a critical value.",
            },
        ),
        Ok(rows) => {
            ui.scope(|ui| {
                ui.set_max_width(420.0);
                widgets::card(ui, pal, |ui| {
                    ui.spacing_mut().item_spacing.y = 3.0;
                    for (l, val) in rows {
                        widgets::row(ui, pal, &l, &val);
                    }
                });
            });
        }
    }
}
