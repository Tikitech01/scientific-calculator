//! Statistics ▸ Tests: t, z, F, χ², ANOVA and correlation tests.

use crate::engine::{self, Env};
use crate::fmt::{self, Fmt};
use crate::hypothesis::{self as h, Outcome, Tail};
use crate::theme::{self, Pal};
use crate::widgets;
use eframe::egui::{self, vec2, Align2, CornerRadius, Frame, Margin, RichText, ScrollArea, Sense, Stroke, TextEdit, Ui};

pub struct TestsState {
    pub kind: usize,
    pub t_kind: usize,
    pub pooled: usize,
    pub z_kind: usize,
    pub chi_kind: usize,
    pub corr_kind: usize,
    pub tail: usize,
    pub alpha: String,
    pub mu0: String,
    pub sigma: String,
    pub x1: String,
    pub n1: String,
    pub x2: String,
    pub n2: String,
    pub p0: String,
    pub table: String,
    pub groups: String,
}

impl Default for TestsState {
    fn default() -> Self {
        TestsState {
            kind: 0,
            t_kind: 0,
            pooled: 0,
            z_kind: 0,
            chi_kind: 0,
            corr_kind: 0,
            tail: 0,
            alpha: "0.05".into(),
            mu0: "0".into(),
            sigma: String::new(),
            x1: String::new(),
            n1: String::new(),
            x2: String::new(),
            n2: String::new(),
            p0: "0.5".into(),
            table: String::new(),
            groups: String::new(),
        }
    }
}

const KINDS: [&str; 6] = ["t-test", "z-test", "F-test", "χ² test", "ANOVA", "Correlation"];

fn label(ui: &mut Ui, pal: &Pal, text: &str, w: f32) {
    let (r, _) = ui.allocate_exact_size(vec2(w, 30.0), Sense::hover());
    ui.painter().text(r.left_center(), Align2::LEFT_CENTER, text, theme::regular(13.5), pal.dim);
}

fn num_field(ui: &mut Ui, pal: &Pal, name: &str, text: &mut String, hint: &str, tip: &str) {
    ui.horizontal(|ui| {
        label(ui, pal, name, 150.0);
        widgets::field(ui, pal, text, hint, 110.0).on_hover_text(tip);
    });
}

/// Numbers in free text: separated by spaces, commas, semicolons or tabs.
fn parse_numbers(line: &str, env: &mut Env) -> Result<Vec<f64>, String> {
    line.split(|c: char| c == ',' || c == ';' || c == '\t' || c.is_whitespace())
        .filter(|t| !t.trim().is_empty())
        .map(|t| match engine::field_value(t, env)? {
            Some(v) => Ok(v),
            None => Err(format!("“{}” is not a number", t)),
        })
        .collect()
}

fn parse_lines(text: &str, env: &mut Env) -> Result<Vec<Vec<f64>>, String> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .enumerate()
        .map(|(i, l)| parse_numbers(l, env).map_err(|e| format!("Line {}: {}", i + 1, e)))
        .collect()
}

fn multiline(ui: &mut Ui, pal: &Pal, text: &mut String, hint: &str, tip: &str) {
    Frame::new()
        .fill(pal.surface)
        .stroke(Stroke::new(1.0_f32, pal.line))
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::symmetric(9, 6))
        .show(ui, |ui| {
            ui.add(
                TextEdit::multiline(text)
                    .frame(false)
                    .desired_rows(5)
                    .desired_width(ui.available_width().min(460.0))
                    .font(theme::regular(14.0))
                    .text_color(pal.text)
                    .hint_text(RichText::new(hint).color(pal.faint)),
            )
            .on_hover_text(tip);
        });
}

fn hypotheses(ui: &mut Ui, pal: &Pal, param: &str, value: &str, tail: Tail) {
    let (h1, sym) = match tail {
        Tail::Two => ("≠", "two-sided"),
        Tail::Less => ("<", "left-tailed"),
        Tail::Greater => (">", "right-tailed"),
    };
    ui.label(
        RichText::new(format!("H₀: {} = {}     H₁: {} {} {}   ({})", param, value, param, h1, value, sym))
            .font(theme::regular(13.0))
            .color(pal.dim),
    );
}

fn tail_and_alpha(ui: &mut Ui, st: &mut TestsState, pal: &Pal, show_tail: bool) {
    ui.horizontal(|ui| {
        if show_tail {
            label(ui, pal, "Alternative", 90.0);
            widgets::segmented(ui, pal, &mut st.tail, &["≠  two-sided", "<  left", ">  right"]);
            ui.add_space(14.0);
        }
        label(ui, pal, "α", 16.0);
        widgets::field(ui, pal, &mut st.alpha, "0.05", 54.0)
            .on_hover_text("Significance level, e.g. 0.05 for 5% or 0.01 for 1%. The confidence level is 1 − α.");
    });
}

fn decision_card(ui: &mut Ui, pal: &Pal, f: Fmt, o: &Outcome, alpha: f64) {
    let v = |x: f64| fmt::real(x, f);
    let reject = o.p < alpha;
    ui.add_space(6.0);
    widgets::card(ui, pal, |ui| {
        ui.set_min_width(380.0);
        ui.spacing_mut().item_spacing.y = 3.0;
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{} = {}", o.statistic.0, v(o.statistic.1))).font(theme::regular(20.0)));
            ui.add_space(18.0);
            ui.label(RichText::new(format!("p = {}", p_text(o.p, f))).font(theme::regular(20.0)));
        });
        let (txt, color) = if reject {
            (format!("Reject H₀ at α = {}  (p < α)", fmt::short(alpha, 4)), pal.accent)
        } else {
            (format!("Do not reject H₀ at α = {}  (p ≥ α)", fmt::short(alpha, 4)), pal.dim)
        };
        ui.label(RichText::new(txt).font(theme::medium(13.5)).color(color));
        ui.add_space(6.0);
        for (l, val) in &o.rows {
            widgets::row(ui, pal, l, &v(*val));
        }
        if let Some(c) = &o.critical {
            widgets::row(ui, pal, "Critical value", c);
        }
        if let Some((l, lo, hi)) = &o.ci {
            widgets::row(ui, pal, l, &format!("({}, {})", v(*lo), v(*hi)));
        }
        if let Some(n) = &o.note {
            ui.add_space(4.0);
            ui.label(RichText::new(n).font(theme::regular(12.5)).color(pal.faint));
        }
    });
}

fn p_text(p: f64, f: Fmt) -> String {
    if p < 1e-4 && p > 0.0 {
        fmt::short(p, 4)
    } else {
        fmt::real(p, f)
    }
}

pub fn ui(ui: &mut Ui, st: &mut TestsState, x: &[f64], y: &[f64], env: &mut Env, f: Fmt, pal: &Pal) {
    ScrollArea::vertical().id_salt("tests").auto_shrink([false, false]).show(ui, |ui| {
        ui.set_max_width(640.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            for (i, k) in KINDS.iter().enumerate() {
                if widgets::tab(ui, pal, k, st.kind == i) {
                    st.kind = i;
                }
            }
        });
        ui.add_space(10.0);
        let alpha = match engine::field_value(&st.alpha, env) {
            Ok(Some(a)) if a > 0.0 && a < 1.0 => a,
            _ => 0.05,
        };
        let tail = [Tail::Two, Tail::Less, Tail::Greater][st.tail.min(2)];
        let result: Option<Result<Outcome, String>> = match st.kind {
            0 => t_test(ui, st, x, y, env, pal, tail, alpha),
            1 => z_test(ui, st, x, env, pal, tail, alpha),
            2 => {
                widgets::dim(ui, pal, "Compares the variances of the X and Y columns.");
                tail_and_alpha(ui, st, pal, true);
                hypotheses(ui, pal, "σ₁²/σ₂²", "1", tail);
                if x.len() < 2 || y.len() < 2 {
                    None
                } else {
                    Some(h::f_test(x, y, tail, alpha))
                }
            }
            3 => return chi_test(ui, st, x, y, env, f, pal, alpha),
            4 => return anova(ui, st, x, y, env, f, pal, alpha),
            _ => {
                widgets::segmented(ui, pal, &mut st.corr_kind, &["Pearson r", "Spearman rank ρ"]);
                ui.add_space(6.0);
                widgets::dim(ui, pal, "Tests whether the X and Y columns are related.");
                tail_and_alpha(ui, st, pal, true);
                hypotheses(ui, pal, "ρ", "0", tail);
                if x.len() < 3 {
                    None
                } else {
                    Some(h::correlation(x, y, st.corr_kind == 1, tail, alpha))
                }
            }
        };
        match result {
            None => {
                ui.add_space(6.0);
                widgets::dim(ui, pal, "Enter the data in the X and Y columns on the left.");
            }
            Some(Err(e)) => {
                ui.add_space(6.0);
                widgets::error(ui, pal, e);
            }
            Some(Ok(o)) => decision_card(ui, pal, f, &o, alpha),
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn t_test(ui: &mut Ui, st: &mut TestsState, x: &[f64], y: &[f64], env: &mut Env, pal: &Pal, tail: Tail, alpha: f64) -> Option<Result<Outcome, String>> {
    widgets::segmented(ui, pal, &mut st.t_kind, &["One sample", "Two samples", "Paired"]);
    ui.add_space(6.0);
    match st.t_kind {
        0 => {
            widgets::dim(ui, pal, "Is the mean of the X column different from μ₀?");
            num_field(ui, pal, "Hypothesised mean μ₀", &mut st.mu0, "0", "The mean under the null hypothesis.");
            tail_and_alpha(ui, st, pal, true);
            hypotheses(ui, pal, "μ", &st.mu0.clone(), tail);
            let mu0 = match engine::field_value(&st.mu0, env) {
                Ok(v) => v.unwrap_or(0.0),
                Err(e) => return Some(Err(e)),
            };
            if x.is_empty() {
                return None;
            }
            Some(h::t_one_sample(x, mu0, tail, alpha))
        }
        1 => {
            widgets::dim(ui, pal, "Do the X and Y columns come from populations with different means?");
            ui.horizontal(|ui| {
                label(ui, pal, "Variances", 90.0);
                widgets::segmented(ui, pal, &mut st.pooled, &["Unequal (Welch)", "Equal (pooled)"]);
            });
            tail_and_alpha(ui, st, pal, true);
            hypotheses(ui, pal, "μ₁ − μ₂", "0", tail);
            if x.is_empty() || y.is_empty() {
                return None;
            }
            Some(h::t_two_sample(x, y, st.pooled == 1, tail, alpha))
        }
        _ => {
            widgets::dim(ui, pal, "Before/after or matched pairs: X and Y on the same row belong together.");
            tail_and_alpha(ui, st, pal, true);
            hypotheses(ui, pal, "μ_d", "0", tail);
            if x.is_empty() || y.is_empty() {
                return None;
            }
            Some(h::t_paired(x, y, tail, alpha))
        }
    }
}

fn z_test(ui: &mut Ui, st: &mut TestsState, x: &[f64], env: &mut Env, pal: &Pal, tail: Tail, alpha: f64) -> Option<Result<Outcome, String>> {
    widgets::segmented(ui, pal, &mut st.z_kind, &["One mean (σ known)", "One proportion", "Two proportions"]);
    ui.add_space(6.0);
    let mut get = |s: &str| engine::field_value(s, env);
    match st.z_kind {
        0 => {
            widgets::dim(ui, pal, "Uses the X column and a known population standard deviation.");
            num_field(ui, pal, "Hypothesised mean μ₀", &mut st.mu0, "0", "The mean under the null hypothesis.");
            num_field(ui, pal, "Population σ", &mut st.sigma, "", "The known population standard deviation.");
            tail_and_alpha(ui, st, pal, true);
            hypotheses(ui, pal, "μ", &st.mu0.clone(), tail);
            let (mu0, sigma) = match (get(&st.mu0), get(&st.sigma)) {
                (Ok(m), Ok(s)) => (m.unwrap_or(0.0), s),
                (Err(e), _) | (_, Err(e)) => return Some(Err(e)),
            };
            let sigma = sigma?;
            if x.is_empty() {
                return None;
            }
            Some(h::z_one_mean(x, mu0, sigma, tail, alpha))
        }
        1 => {
            num_field(ui, pal, "Successes  x", &mut st.x1, "", "How many in the sample have the property.");
            num_field(ui, pal, "Sample size  n", &mut st.n1, "", "Size of the sample.");
            num_field(ui, pal, "Hypothesised p₀", &mut st.p0, "0.5", "The proportion under the null hypothesis.");
            tail_and_alpha(ui, st, pal, true);
            hypotheses(ui, pal, "p", &st.p0.clone(), tail);
            match (get(&st.x1), get(&st.n1), get(&st.p0)) {
                (Ok(Some(x1)), Ok(Some(n1)), Ok(Some(p0))) => Some(h::z_one_prop(x1, n1, p0, tail, alpha)),
                (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => Some(Err(e)),
                _ => {
                    widgets::dim(ui, pal, "Enter the successes, the sample size and p₀.");
                    None
                }
            }
        }
        _ => {
            ui.horizontal(|ui| {
                label(ui, pal, "Group 1: successes", 150.0);
                widgets::field(ui, pal, &mut st.x1, "x₁", 70.0);
                label(ui, pal, "of", 20.0);
                widgets::field(ui, pal, &mut st.n1, "n₁", 70.0);
            });
            ui.horizontal(|ui| {
                label(ui, pal, "Group 2: successes", 150.0);
                widgets::field(ui, pal, &mut st.x2, "x₂", 70.0);
                label(ui, pal, "of", 20.0);
                widgets::field(ui, pal, &mut st.n2, "n₂", 70.0);
            });
            tail_and_alpha(ui, st, pal, true);
            hypotheses(ui, pal, "p₁ − p₂", "0", tail);
            match (get(&st.x1), get(&st.n1), get(&st.x2), get(&st.n2)) {
                (Ok(Some(a)), Ok(Some(b)), Ok(Some(c)), Ok(Some(d))) => Some(h::z_two_prop(a, b, c, d, tail, alpha)),
                (Err(e), ..) | (_, Err(e), ..) | (_, _, Err(e), _) | (.., Err(e)) => Some(Err(e)),
                _ => {
                    widgets::dim(ui, pal, "Enter successes and sample size for both groups.");
                    None
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn chi_test(ui: &mut Ui, st: &mut TestsState, x: &[f64], y: &[f64], env: &mut Env, f: Fmt, pal: &Pal, alpha: f64) {
    widgets::segmented(ui, pal, &mut st.chi_kind, &["Goodness of fit", "Independence (contingency table)"]);
    ui.add_space(6.0);
    tail_and_alpha(ui, st, pal, false);
    if st.chi_kind == 0 {
        widgets::dim(ui, pal, "Observed counts in the X column. Expected counts or proportions in Y (leave Y empty for equal expected counts).");
        ui.label(RichText::new("H₀: the data follow the expected distribution").font(theme::regular(13.0)).color(pal.dim));
        if x.is_empty() {
            ui.add_space(6.0);
            widgets::dim(ui, pal, "Enter the observed counts in the X column on the left.");
            return;
        }
        match h::chi2_gof(x, if y.is_empty() { None } else { Some(y) }, alpha) {
            Ok(o) => decision_card(ui, pal, f, &o, alpha),
            Err(e) => widgets::error(ui, pal, e),
        }
        return;
    }
    widgets::dim(ui, pal, "Type the table of observed counts: one row per line, values separated by spaces or commas.");
    multiline(ui, pal, &mut st.table, "e.g.\n20 30 25\n15 35 40", "Each line is a row of the contingency table. Separate the counts with spaces, commas or tabs.");
    ui.label(RichText::new("H₀: the row and column variables are independent").font(theme::regular(13.0)).color(pal.dim));
    if st.table.trim().is_empty() {
        return;
    }
    let table = match parse_lines(&st.table, env) {
        Ok(t) => t,
        Err(e) => {
            widgets::error(ui, pal, e);
            return;
        }
    };
    match h::chi2_independence(&table, alpha) {
        Ok((mut o, expected)) => {
            // Show the size as rows × columns instead of a number.
            let size = format!("{} × {}", table.len(), table[0].len());
            o.rows.remove(0);
            decision_card(ui, pal, f, &o, alpha);
            ui.add_space(8.0);
            widgets::caption(ui, pal, &format!("EXPECTED COUNTS  ·  {} table", size));
            ui.add_space(4.0);
            grid(ui, pal, &expected);
        }
        Err(e) => widgets::error(ui, pal, e),
    }
}

fn grid(ui: &mut Ui, pal: &Pal, rows: &[Vec<f64>]) {
    widgets::card(ui, pal, |ui| {
        egui::Grid::new(ui.next_auto_id()).spacing(vec2(18.0, 4.0)).show(ui, |ui| {
            for r in rows {
                for v in r {
                    ui.label(RichText::new(fmt::short(*v, 5)).font(theme::regular(13.5)));
                }
                ui.end_row();
            }
        });
    });
}

#[allow(clippy::too_many_arguments)]
fn anova(ui: &mut Ui, st: &mut TestsState, x: &[f64], y: &[f64], env: &mut Env, f: Fmt, pal: &Pal, alpha: f64) {
    widgets::dim(ui, pal, "One-way ANOVA: are the means of several groups equal? Type one group per line, or leave the box empty to compare the X and Y columns.");
    multiline(ui, pal, &mut st.groups, "e.g.\n23 25 21 27\n30 28 33 29\n18 22 20 19", "Each line is one group. Separate the values with spaces, commas or tabs.");
    tail_and_alpha(ui, st, pal, false);
    ui.label(RichText::new("H₀: all group means are equal").font(theme::regular(13.0)).color(pal.dim));
    let groups: Vec<Vec<f64>> = if st.groups.trim().is_empty() {
        if x.is_empty() || y.is_empty() {
            ui.add_space(6.0);
            widgets::dim(ui, pal, "Enter the groups above, or data in both the X and Y columns.");
            return;
        }
        vec![x.to_vec(), y.to_vec()]
    } else {
        match parse_lines(&st.groups, env) {
            Ok(g) => g,
            Err(e) => {
                widgets::error(ui, pal, e);
                return;
            }
        }
    };
    let a = match h::anova_one_way(&groups, alpha) {
        Ok(a) => a,
        Err(e) => {
            widgets::error(ui, pal, e);
            return;
        }
    };
    let v = |x: f64| fmt::real(x, f);
    let reject = a.p < alpha;
    ui.add_space(6.0);
    widgets::card(ui, pal, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("F = {}", v(a.f))).font(theme::regular(20.0)));
            ui.add_space(18.0);
            ui.label(RichText::new(format!("p = {}", p_text(a.p, f))).font(theme::regular(20.0)));
        });
        let (txt, color) = if reject {
            (format!("Reject H₀ at α = {}: at least one group mean differs", fmt::short(alpha, 4)), pal.accent)
        } else {
            (format!("Do not reject H₀ at α = {}", fmt::short(alpha, 4)), pal.dim)
        };
        ui.label(RichText::new(txt).font(theme::medium(13.5)).color(color));
        ui.add_space(8.0);
        // The ANOVA table.
        let ms_b = a.ss_between / a.df_between;
        let ms_w = a.ss_within / a.df_within;
        let head = |ui: &mut Ui, t: &str| {
            ui.label(RichText::new(t).font(theme::medium(12.5)).color(pal.dim));
        };
        let cell = |ui: &mut Ui, t: String| {
            ui.label(RichText::new(t).font(theme::regular(13.5)));
        };
        egui::Grid::new("anova_table").spacing(vec2(20.0, 5.0)).striped(false).show(ui, |ui| {
            for t in ["Source", "SS", "df", "MS", "F", "p"] {
                head(ui, t);
            }
            ui.end_row();
            cell(ui, "Between groups".into());
            cell(ui, v(a.ss_between));
            cell(ui, v(a.df_between));
            cell(ui, v(ms_b));
            cell(ui, v(a.f));
            cell(ui, p_text(a.p, f));
            ui.end_row();
            cell(ui, "Within groups".into());
            cell(ui, v(a.ss_within));
            cell(ui, v(a.df_within));
            cell(ui, v(ms_w));
            cell(ui, String::new());
            cell(ui, String::new());
            ui.end_row();
            cell(ui, "Total".into());
            cell(ui, v(a.ss_between + a.ss_within));
            cell(ui, v(a.df_between + a.df_within));
            ui.end_row();
        });
        ui.add_space(6.0);
        widgets::row(ui, pal, "Critical value", &format!("F = {}", fmt::short(a.f_critical, 6)));
        widgets::row(ui, pal, "η² (effect size)", &v(a.ss_between / (a.ss_between + a.ss_within)));
        widgets::row(ui, pal, "Grand mean", &v(a.grand_mean));
    });
    ui.add_space(8.0);
    widgets::caption(ui, pal, "GROUPS");
    ui.add_space(4.0);
    widgets::card(ui, pal, |ui| {
        egui::Grid::new("anova_groups").spacing(vec2(20.0, 4.0)).show(ui, |ui| {
            for t in ["Group", "n", "Mean", "Std deviation"] {
                ui.label(RichText::new(t).font(theme::medium(12.5)).color(pal.dim));
            }
            ui.end_row();
            for (i, (n, m, sd)) in a.groups.iter().enumerate() {
                ui.label(RichText::new((i + 1).to_string()).font(theme::regular(13.5)));
                ui.label(RichText::new(n.to_string()).font(theme::regular(13.5)));
                ui.label(RichText::new(v(*m)).font(theme::regular(13.5)));
                ui.label(RichText::new(if sd.is_finite() { v(*sd) } else { "–".into() }).font(theme::regular(13.5)));
                ui.end_row();
            }
        });
    });
}
