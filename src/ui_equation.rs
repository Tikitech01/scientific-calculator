//! Equation mode: polynomial roots, simultaneous linear equations, numeric solver.

use crate::engine::{self, Env};
use crate::fmt::{self, Fmt};
use crate::help;
use crate::numeric::{self, LinSol};
use crate::theme::{self, Pal};
use crate::widgets;
use eframe::egui::{self, vec2, Align, Align2, RichText, ScrollArea, Sense, Ui};

pub struct EqState {
    pub sub: usize,
    pub deg: usize,
    pub coef: Vec<String>,
    pub n: usize,
    pub sys: Vec<Vec<String>>,
    pub equation: String,
    pub lo: String,
    pub hi: String,
}

impl Default for EqState {
    fn default() -> Self {
        EqState {
            sub: 0,
            deg: 0,
            coef: vec![String::new(); 7],
            n: 0,
            sys: vec![vec![String::new(); 7]; 6],
            equation: String::new(),
            lo: "-100".into(),
            hi: "100".into(),
        }
    }
}

const SUB: [&str; 6] = ["₁", "₂", "₃", "₄", "₅", "₆"];
const VARS: [&str; 6] = ["x", "y", "z", "w", "u", "v"];
const POW: [&str; 7] = ["", "x", "x²", "x³", "x⁴", "x⁵", "x⁶"];

fn term(ui: &mut Ui, pal: &Pal, text: &str) {
    let w = widgets::text_width(ui, text, theme::regular(15.0)) + 4.0;
    let (r, _) = ui.allocate_exact_size(vec2(w, 32.0), Sense::hover());
    ui.painter().text(r.left_center(), Align2::LEFT_CENTER, text, theme::regular(15.0), pal.dim);
}

fn cell(ui: &mut Ui, pal: &Pal, text: &mut String, hint: &str) -> egui::Response {
    widgets::field_with(ui, pal, text, hint, 58.0, Align::Center, None)
        .on_hover_text("A number or a calculation, e.g. 2.5, 1/3, sqrt(2). Empty counts as 0.")
}

fn result_line(ui: &mut Ui, pal: &Pal, name: &str, value: &str, alt: Option<String>) {
    ui.horizontal(|ui| {
        ui.set_min_height(28.0);
        ui.label(RichText::new(format!("{} =", name)).color(pal.dim).size(15.0));
        ui.add(egui::Label::new(RichText::new(value).font(theme::regular(19.0))).selectable(true));
        if let Some(alt) = alt {
            ui.label(RichText::new(format!("=  {}", alt)).color(pal.faint).size(14.0));
        }
    });
}

/// Read a row of fields; empty fields count as 0. `None` when every field is empty.
fn read(fields: &[String], env: &mut Env) -> Result<Option<Vec<f64>>, String> {
    let mut out = Vec::with_capacity(fields.len());
    let mut any = false;
    for f in fields {
        match engine::field_value(f, env)? {
            Some(v) => {
                any = true;
                out.push(v);
            }
            None => out.push(0.0),
        }
    }
    Ok(if any { Some(out) } else { None })
}

pub fn ui(ui: &mut Ui, st: &mut EqState, env: &mut Env, f: Fmt, pal: &Pal) {
    widgets::segmented(ui, pal, &mut st.sub, &["Polynomial", "Simultaneous", "Solver"]);
    ui.add_space(14.0);
    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match st.sub {
        0 => polynomial(ui, st, env, f, pal),
        1 => simultaneous(ui, st, env, f, pal),
        _ => solver(ui, st, env, f, pal),
    });
}

fn polynomial(ui: &mut Ui, st: &mut EqState, env: &mut Env, f: Fmt, pal: &Pal) {
    ui.horizontal(|ui| {
        widgets::dim(ui, pal, "Degree");
        widgets::segmented(ui, pal, &mut st.deg, &["2", "3", "4", "5", "6"]);
    });
    ui.add_space(10.0);
    let deg = st.deg + 2;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        for k in 0..=deg {
            let p = deg - k;
            cell(ui, pal, &mut st.coef[k], "0");
            let tail = if p == 0 { "= 0".to_string() } else { format!("{}  +", POW[p]) };
            term(ui, pal, &tail);
        }
    });
    ui.add_space(16.0);
    match read(&st.coef[..=deg], env) {
        Err(e) => widgets::error(ui, pal, e),
        Ok(None) => widgets::dim(ui, pal, "Enter the coefficients. Each field accepts an expression such as 1/3 or √2."),
        Ok(Some(c)) => match numeric::poly_roots(&c) {
            Err(e) => widgets::error(ui, pal, e),
            Ok(roots) => {
                widgets::card(ui, pal, |ui| {
                    ui.set_min_width(320.0);
                    for (i, z) in roots.iter().enumerate() {
                        let alt = if z.im == 0.0 { fmt::exact(z.re) } else { None };
                        result_line(ui, pal, &format!("x{}", SUB[i]), &fmt::complex(*z, f), alt);
                    }
                });
            }
        },
    }
}

fn simultaneous(ui: &mut Ui, st: &mut EqState, env: &mut Env, f: Fmt, pal: &Pal) {
    ui.horizontal(|ui| {
        widgets::dim(ui, pal, "Unknowns");
        widgets::segmented(ui, pal, &mut st.n, &["2", "3", "4", "5", "6"]);
    });
    ui.add_space(10.0);
    let n = st.n + 2;
    for i in 0..n {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for j in 0..n {
                cell(ui, pal, &mut st.sys[i][j], "0");
                term(ui, pal, &format!("{}  {}", VARS[j], if j + 1 == n { "=" } else { "+" }));
            }
            cell(ui, pal, &mut st.sys[i][6], "0");
        });
    }
    ui.add_space(16.0);
    let mut a = Vec::new();
    let mut b = Vec::new();
    let mut any = false;
    for i in 0..n {
        let mut fields: Vec<String> = st.sys[i][..n].to_vec();
        fields.push(st.sys[i][6].clone());
        match read(&fields, env) {
            Err(e) => {
                widgets::error(ui, pal, e);
                return;
            }
            Ok(None) => {
                a.push(vec![0.0; n]);
                b.push(0.0);
            }
            Ok(Some(row)) => {
                any = true;
                a.push(row[..n].to_vec());
                b.push(row[n]);
            }
        }
    }
    if !any {
        widgets::dim(ui, pal, "Enter the coefficients of each equation.");
        return;
    }
    match numeric::linear_solve(&a, &b) {
        LinSol::Unique(x) => {
            widgets::card(ui, pal, |ui| {
                ui.set_min_width(320.0);
                for (i, v) in x.iter().enumerate() {
                    result_line(ui, pal, VARS[i], &fmt::real(*v, f), fmt::exact(*v));
                }
            });
        }
        LinSol::NoSolution => widgets::dim(ui, pal, "No solution. The equations contradict each other."),
        LinSol::Infinite => widgets::dim(ui, pal, "Infinitely many solutions. The equations are not independent."),
    }
}

fn solver(ui: &mut Ui, st: &mut EqState, env: &mut Env, f: Fmt, pal: &Pal) {
    widgets::field(ui, pal, &mut st.equation, "x^3 - 2x = 5", (ui.available_width() - 40.0).min(520.0))
        .on_hover_text(help::EQUATION);
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        widgets::dim(ui, pal, "Search x from");
        widgets::field(ui, pal, &mut st.lo, "", 70.0).on_hover_text("Smallest x to search. Solutions outside the range are not listed.");
        widgets::dim(ui, pal, "to");
        widgets::field(ui, pal, &mut st.hi, "", 70.0).on_hover_text("Largest x to search.");
    });
    ui.add_space(16.0);
    let src = st.equation.trim();
    if src.is_empty() {
        widgets::dim(ui, pal, "Enter an equation in x. Every solution inside the search range is listed.");
        return;
    }
    let parsed = (|| -> Result<_, String> {
        let (l, r) = match src.split_once('=') {
            Some((l, r)) => (engine::parse(l)?, Some(engine::parse(r)?)),
            None => (engine::parse(src)?, None),
        };
        let lo = engine::field_value(&st.lo, env)?.ok_or("Enter the search range")?;
        let hi = engine::field_value(&st.hi, env)?.ok_or("Enter the search range")?;
        if hi <= lo {
            return Err("The range must go from a smaller to a larger value".into());
        }
        Ok((l, r, lo, hi))
    })();
    let (l, r, lo, hi) = match parsed {
        Ok(p) => p,
        Err(e) => {
            widgets::error(ui, pal, e);
            return;
        }
    };
    let roots = engine::with_var(env, "x", |env| {
        numeric::find_roots(
            &mut |x| {
                let a = engine::eval_at(&l, env, "x", x);
                match &r {
                    Some(r) => a - engine::eval_at(r, env, "x", x),
                    None => a,
                }
            },
            lo,
            hi,
            6000,
        )
    });
    if roots.is_empty() {
        widgets::dim(ui, pal, "No real solution in this range.");
        return;
    }
    widgets::card(ui, pal, |ui| {
        ui.set_min_width(320.0);
        for (i, x) in roots.iter().take(40).enumerate() {
            let name = if roots.len() == 1 { "x".to_string() } else { format!("x{}", sub(i + 1)) };
            result_line(ui, pal, &name, &fmt::real(*x, f), fmt::exact(*x));
        }
        if roots.len() > 40 {
            widgets::dim(ui, pal, format!("and {} more", roots.len() - 40));
        }
    });
}

fn sub(n: usize) -> String {
    n.to_string()
        .chars()
        .map(|c| char::from_u32('₀' as u32 + c.to_digit(10).unwrap()).unwrap())
        .collect()
}
