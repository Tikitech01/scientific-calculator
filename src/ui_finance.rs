//! Finance mode: time value of money, loans, interest, cash flows, depreciation and business sums.

use crate::engine::{self, Env};
use crate::finance::{self, DepMethod, Tvm, TvmVar};
use crate::fmt::{self, Fmt};
use crate::theme::{self, Pal};
use crate::widgets;
use eframe::egui::{self, vec2, Align, ComboBox, Layout, RichText, ScrollArea, TextEdit, Ui};

const SUBS: [&str; 6] = ["Time value", "Loan / EMI", "Interest", "Cash flow", "Depreciation", "Business"];
const FREQS: [(&str, f64); 7] = [
    ("Yearly", 1.0),
    ("Half-yearly", 2.0),
    ("Quarterly", 4.0),
    ("Monthly", 12.0),
    ("Weekly", 52.0),
    ("Daily", 365.0),
    ("Continuous", 0.0),
];
const PERIODS: [(&str, f64); 5] = [("Monthly", 12.0), ("Quarterly", 4.0), ("Half-yearly", 2.0), ("Yearly", 1.0), ("Weekly", 52.0)];

pub struct FinState {
    pub sub: usize,
    // Time value of money
    pub tvm: [String; 5],
    pub tvm_py: usize,
    pub tvm_cy: usize,
    pub tvm_begin: bool,
    tvm_result: Option<(TvmVar, Result<f64, String>)>,
    // Loan
    pub loan: [String; 4],
    pub loan_years: bool,
    pub loan_yearly: bool,
    // Interest
    pub int_kind: usize,
    pub int: [String; 4],
    pub int_freq: usize,
    // Cash flow
    pub cf_rate: String,
    pub cf0: String,
    pub cf_flows: String,
    pub cf_reinvest: String,
    // Depreciation
    pub dep: [String; 4],
    pub dep_method: usize,
    // Business
    pub biz_kind: usize,
    pub biz: [String; 4],
}

impl Default for FinState {
    fn default() -> Self {
        FinState {
            sub: 0,
            tvm: Default::default(),
            tvm_py: 0,
            tvm_cy: 0,
            tvm_begin: false,
            tvm_result: None,
            loan: Default::default(),
            loan_years: true,
            loan_yearly: false,
            int_kind: 0,
            int: Default::default(),
            int_freq: 0,
            cf_rate: "10".into(),
            cf0: String::new(),
            cf_flows: String::new(),
            cf_reinvest: String::new(),
            dep: Default::default(),
            dep_method: 0,
            biz_kind: 0,
            biz: Default::default(),
        }
    }
}

impl FinState {
    /// Sample figures for screenshots and a first look.
    pub fn load_demo(&mut self) {
        self.tvm = ["240", "8.5", "2500000", "", "0"].map(String::from);
        let t = Tvm { n: 240.0, rate: 8.5, pv: 2_500_000.0, pmt: 0.0, fv: 0.0, py: 12.0, cy: 12.0, begin: false };
        let pmt = t.solve(TvmVar::Pmt);
        if let Ok(x) = pmt {
            self.tvm[3] = format!("{:.2}", x);
        }
        self.tvm_result = Some((TvmVar::Pmt, pmt));
        self.loan = ["2500000", "8.5", "20", ""].map(String::from);
        self.int = ["100000", "7.5", "10", ""].map(String::from);
        self.int_freq = 2;
        self.cf0 = "-500000".into();
        self.cf_flows = "150000\n180000\n200000\n220000".into();
        self.cf_reinvest = "8".into();
        self.dep = ["1200000", "200000", "5", "200"].map(String::from);
        self.dep_method = 2;
        self.biz = ["300000", "250", "150", ""].map(String::from);
    }
}

fn money(x: f64) -> String {
    fmt::real(x, Fmt::Fix(2))
}

fn pct(x: f64) -> String {
    format!("{}%", fmt::real(x, Fmt::Fix(4)).trim_end_matches('0').trim_end_matches('.'))
}

fn value(ui: &mut Ui, pal: &Pal, src: &str, env: &mut Env) -> Option<f64> {
    match engine::field_value(src, env) {
        Ok(v) => v,
        Err(e) => {
            widgets::error(ui, pal, e);
            None
        }
    }
}

fn labelled(ui: &mut Ui, pal: &Pal, label: &str, text: &mut String, hint: &str, tip: &str) -> egui::Response {
    let mut resp = None;
    ui.horizontal(|ui| {
        crate::widgets::row_label(ui, RichText::new(label).size(13.5).color(pal.dim), 150.0, 34.0);
        resp = Some(widgets::field(ui, pal, text, hint, 150.0).on_hover_text(tip));
    });
    resp.unwrap()
}

fn choice(ui: &mut Ui, pal: &Pal, label: &str, sel: &mut usize, names: &[&str], id: &str) {
    ui.horizontal(|ui| {
        crate::widgets::row_label(ui, RichText::new(label).size(13.5).color(pal.dim), 150.0, 34.0);
        ui.spacing_mut().interact_size.y = 28.0;
        ComboBox::from_id_salt(id).width(170.0).selected_text(names[*sel]).show_ui(ui, |ui| {
            for (i, n) in names.iter().enumerate() {
                ui.selectable_value(sel, i, *n);
            }
        });
    });
}

fn big(ui: &mut Ui, pal: &Pal, label: &str, val: &str) {
    ui.label(RichText::new(label).size(12.5).color(pal.dim));
    ui.add(egui::Label::new(RichText::new(val).font(theme::light(28.0)).color(pal.text)).selectable(true));
}

fn table<const N: usize>(ui: &mut Ui, pal: &Pal, id: &str, head: [&str; N], rows: &[[String; N]]) {
    widgets::copy_table(ui, pal, &head, || rows.iter().map(|r| r.to_vec()).collect());
    ScrollArea::vertical().id_salt(id).max_height(320.0).auto_shrink([false, true]).show(ui, |ui| {
        egui::Grid::new(id).striped(true).spacing(vec2(18.0, 4.0)).min_col_width(105.0).show(ui, |ui| {
            for h in head {
                ui.label(RichText::new(h).font(theme::medium(12.5)).color(pal.dim));
            }
            ui.end_row();
            for r in rows {
                for (i, c) in r.iter().enumerate() {
                    if i == 0 {
                        ui.label(RichText::new(c).size(13.0).color(pal.dim));
                    } else {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.add(egui::Label::new(RichText::new(c).size(13.0)).selectable(true));
                        });
                    }
                }
                ui.end_row();
            }
        });
    });
}

pub fn ui(ui: &mut Ui, st: &mut FinState, env: &mut Env, pal: &Pal) {
    widgets::segmented(ui, pal, &mut st.sub, &SUBS);
    ui.add_space(14.0);
    ScrollArea::vertical().id_salt("finance").auto_shrink([false, false]).show(ui, |ui| {
        ui.set_max_width(ui.available_width().min(820.0));
        ui.spacing_mut().item_spacing.y = 6.0;
        match st.sub {
            0 => tvm_ui(ui, st, env, pal),
            1 => loan_ui(ui, st, env, pal),
            2 => interest_ui(ui, st, env, pal),
            3 => cash_flow_ui(ui, st, env, pal),
            4 => depreciation_ui(ui, st, env, pal),
            _ => business_ui(ui, st, env, pal),
        }
    });
}

// ---------------------------------------------------------------- Time value of money

fn tvm_ui(ui: &mut Ui, st: &mut FinState, env: &mut Env, pal: &Pal) {
    widgets::dim(ui, pal, "Fill in four of N, I/Y, PV, PMT and FV, then press Solve next to the fifth. Money you pay is negative, money you receive is positive.");
    ui.add_space(6.0);
    let rows: [(&str, &str, &str, TvmVar); 5] = [
        ("N", "Number of payments", "Total number of payments, e.g. 240 for 20 years of monthly payments.", TvmVar::N),
        ("I/Y", "Annual rate %", "Nominal interest rate per year, in percent.", TvmVar::Rate),
        ("PV", "Present value", "Value today. A loan you receive is positive; a deposit you make is negative.", TvmVar::Pv),
        ("PMT", "Payment", "Regular payment each period. Payments you make are negative.", TvmVar::Pmt),
        ("FV", "Future value", "Value at the end. 0 for a loan paid off in full.", TvmVar::Fv),
    ];
    let mut solve = None;
    let mut parsed = [None; 5];
    for (i, (name, hint, tip, var)) in rows.iter().enumerate() {
        ui.horizontal(|ui| {
            crate::widgets::row_label(ui, RichText::new(*name).font(theme::medium(14.0)), 60.0, 24.0);
            widgets::field(ui, pal, &mut st.tvm[i], hint, 170.0).on_hover_text(*tip);
            if ui.add(egui::Button::new(RichText::new("Solve").size(12.5))).on_hover_text(format!("Work out {} from the other four", name)).clicked() {
                solve = Some(*var);
            }
            if let Some((v, Ok(_))) = &st.tvm_result {
                if v == var {
                    ui.label(RichText::new("←  solved").size(12.0).color(pal.accent));
                }
            }
        });
        parsed[i] = match engine::field_value(&st.tvm[i], env) {
            Ok(v) => v,
            Err(_) => None,
        };
    }
    ui.add_space(4.0);
    let names: Vec<&str> = PERIODS.iter().map(|p| p.0).collect();
    choice(ui, pal, "Payments per year", &mut st.tvm_py, &names, "tvm_py");
    choice(ui, pal, "Compounding", &mut st.tvm_cy, &names, "tvm_cy");
    ui.horizontal(|ui| {
        crate::widgets::row_label(ui, RichText::new("Payments due").size(13.5).color(pal.dim), 150.0, 34.0);
        let mut b = st.tvm_begin as usize;
        if widgets::segmented(ui, pal, &mut b, &["End of period", "Beginning"]) {
            st.tvm_begin = b == 1;
        }
    });
    if let Some(var) = solve {
        let idx = rows.iter().position(|r| r.3 == var).unwrap();
        let missing: Vec<&str> = rows.iter().enumerate().filter(|(i, _)| *i != idx && parsed[*i].is_none()).map(|(_, r)| r.0).collect();
        let res = if !missing.is_empty() {
            Err(format!("Fill in {} first (use 0 if there is none).", missing.join(", ")))
        } else {
            let g = |i: usize| parsed[i].unwrap_or(0.0);
            let t = Tvm { n: g(0), rate: g(1), pv: g(2), pmt: g(3), fv: g(4), py: PERIODS[st.tvm_py].1, cy: PERIODS[st.tvm_cy].1, begin: st.tvm_begin };
            t.solve(var)
        };
        if let Ok(x) = res {
            st.tvm[idx] = fmt::real(x, Fmt::Fix(if var == TvmVar::Rate || var == TvmVar::N { 4 } else { 2 }))
                .replace(['\u{2009}', ','], "")
                .replace('−', "-");
        }
        st.tvm_result = Some((var, res));
    }
    ui.add_space(8.0);
    if let Some((var, res)) = &st.tvm_result {
        widgets::card(ui, pal, |ui| {
            ui.set_min_width(360.0);
            match res {
                Ok(x) => {
                    let name = rows.iter().find(|r| r.3 == *var).map(|r| r.0).unwrap_or("");
                    let text = match var {
                        TvmVar::Rate => pct(*x),
                        TvmVar::N => fmt::real(*x, Fmt::Fix(2)),
                        _ => money(*x),
                    };
                    big(ui, pal, &format!("{} =", name), &text);
                    if *var == TvmVar::N {
                        let py = PERIODS[st.tvm_py].1;
                        widgets::row(ui, pal, "In years", &fmt::real(x / py, Fmt::Fix(2)));
                    }
                    if let (Some(n), Some(pmt)) = (
                        engine::field_value(&st.tvm[0], env).ok().flatten(),
                        engine::field_value(&st.tvm[3], env).ok().flatten(),
                    ) {
                        let pv = engine::field_value(&st.tvm[2], env).ok().flatten().unwrap_or(0.0);
                        let fv = engine::field_value(&st.tvm[4], env).ok().flatten().unwrap_or(0.0);
                        widgets::row(ui, pal, "Total of payments", &money((n * pmt).abs()));
                        widgets::row(ui, pal, "Total interest", &money((n * pmt + pv + fv).abs()));
                    }
                }
                Err(e) => widgets::error(ui, pal, e.clone()),
            }
        });
    }
}

// ---------------------------------------------------------------- Loan

fn loan_ui(ui: &mut Ui, st: &mut FinState, env: &mut Env, pal: &Pal) {
    widgets::dim(ui, pal, "Equal monthly instalments (EMI) and the repayment schedule.");
    ui.add_space(6.0);
    labelled(ui, pal, "Loan amount", &mut st.loan[0], "e.g. 2500000", "Principal borrowed.");
    labelled(ui, pal, "Interest rate % / year", &mut st.loan[1], "e.g. 8.5", "Annual interest rate in percent, reduced monthly.");
    ui.horizontal(|ui| {
        crate::widgets::row_label(ui, RichText::new("Term").size(13.5).color(pal.dim), 150.0, 34.0);
        widgets::field(ui, pal, &mut st.loan[2], if st.loan_years { "years" } else { "months" }, 90.0);
        let mut y = if st.loan_years { 0 } else { 1 };
        if widgets::segmented(ui, pal, &mut y, &["Years", "Months"]) {
            st.loan_years = y == 0;
        }
    });
    labelled(ui, pal, "Extra payment / month", &mut st.loan[3], "optional", "An additional amount paid towards the principal every month. Shortens the loan.");
    ui.add_space(8.0);
    let (Some(p), Some(r), Some(term)) = (value(ui, pal, &st.loan[0], env), value(ui, pal, &st.loan[1], env), value(ui, pal, &st.loan[2], env)) else {
        widgets::dim(ui, pal, "Enter the amount, rate and term.");
        return;
    };
    let extra = value(ui, pal, &st.loan[3], env).unwrap_or(0.0);
    let months = if st.loan_years { term * 12.0 } else { term };
    if p <= 0.0 || months < 1.0 || r < 0.0 {
        widgets::error(ui, pal, "The amount and term must be positive.");
        return;
    }
    let months = months.round() as u32;
    let pay = finance::emi(p, r, months as f64);
    let rows = finance::amortization(p, r, months, extra);
    let interest: f64 = rows.iter().map(|x| x.interest).sum();
    let total: f64 = rows.iter().map(|x| x.payment).sum();
    widgets::card(ui, pal, |ui| {
        ui.set_min_width(360.0);
        big(ui, pal, "Monthly instalment (EMI)", &money(pay));
        widgets::row(ui, pal, "Total interest", &money(interest));
        widgets::row(ui, pal, "Total paid", &money(total));
        widgets::row(ui, pal, "Interest as share of principal", &pct(interest / p * 100.0));
        if extra > 0.0 {
            let saved: f64 = finance::amortization(p, r, months, 0.0).iter().map(|x| x.interest).sum::<f64>() - interest;
            widgets::row(ui, pal, "Paid off in", &format!("{} months ({} saved)", rows.len(), money(saved)));
        }
    });
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        widgets::caption(ui, pal, "REPAYMENT SCHEDULE");
        ui.add_space(10.0);
        let mut y = st.loan_yearly as usize;
        if widgets::segmented(ui, pal, &mut y, &["Monthly", "Yearly"]) {
            st.loan_yearly = y == 1;
        }
    });
    let shown = if st.loan_yearly { finance::yearly(&rows) } else { rows };
    let data: Vec<[String; 5]> = shown
        .iter()
        .map(|x| [x.period.to_string(), money(x.payment), money(x.principal), money(x.interest), money(x.balance)])
        .collect();
    table(ui, pal, "loan_table", [if st.loan_yearly { "Year" } else { "Month" }, "Payment", "Principal", "Interest", "Balance"], &data);
}

// ---------------------------------------------------------------- Interest

fn interest_ui(ui: &mut Ui, st: &mut FinState, env: &mut Env, pal: &Pal) {
    widgets::segmented(ui, pal, &mut st.int_kind, &["Compound", "Simple", "Rate conversion", "Growth (CAGR)"]);
    ui.add_space(10.0);
    match st.int_kind {
        0 | 1 => {
            labelled(ui, pal, "Principal", &mut st.int[0], "e.g. 100000", "Amount invested or borrowed.");
            labelled(ui, pal, "Rate % / year", &mut st.int[1], "e.g. 7.5", "Annual interest rate in percent.");
            labelled(ui, pal, "Time (years)", &mut st.int[2], "e.g. 10", "Length of the investment in years; 18 months is 1.5.");
            if st.int_kind == 0 {
                let names: Vec<&str> = FREQS.iter().map(|f| f.0).collect();
                choice(ui, pal, "Compounded", &mut st.int_freq, &names, "int_freq");
            }
            ui.add_space(8.0);
            let (Some(p), Some(r), Some(t)) = (value(ui, pal, &st.int[0], env), value(ui, pal, &st.int[1], env), value(ui, pal, &st.int[2], env)) else {
                widgets::dim(ui, pal, "Enter the principal, rate and time.");
                return;
            };
            let amount = if st.int_kind == 0 { finance::compound(p, r, t, FREQS[st.int_freq].1) } else { finance::simple(p, r, t) };
            widgets::card(ui, pal, |ui| {
                ui.set_min_width(360.0);
                big(ui, pal, "Final amount", &money(amount));
                widgets::row(ui, pal, "Interest earned", &money(amount - p));
                widgets::row(ui, pal, "Growth", &pct((amount / p - 1.0) * 100.0));
                if st.int_kind == 0 {
                    widgets::row(ui, pal, "Effective annual rate", &pct(finance::effective_rate(r, FREQS[st.int_freq].1)));
                    widgets::row(ui, pal, "Doubling time", &format!("{} years", fmt::real(finance::doubling_time(finance::effective_rate(r, FREQS[st.int_freq].1)), Fmt::Fix(2))));
                }
            });
            if st.int_kind == 0 && t >= 2.0 && t <= 60.0 {
                ui.add_space(10.0);
                widgets::caption(ui, pal, "YEAR BY YEAR");
                let data: Vec<[String; 3]> = (1..=t.ceil() as u32)
                    .map(|y| {
                        let a = finance::compound(p, r, y as f64, FREQS[st.int_freq].1);
                        [y.to_string(), money(a - p), money(a)]
                    })
                    .collect();
                table(ui, pal, "int_table", ["Year", "Interest so far", "Amount"], &data);
            }
        }
        2 => {
            widgets::dim(ui, pal, "Convert between a nominal (quoted) rate and the effective annual rate it really gives.");
            labelled(ui, pal, "Nominal rate %", &mut st.int[0], "e.g. 12", "Quoted annual rate.");
            labelled(ui, pal, "Effective rate %", &mut st.int[3], "e.g. 12.68", "Actual yearly growth after compounding.");
            let names: Vec<&str> = FREQS.iter().map(|f| f.0).collect();
            choice(ui, pal, "Compounded", &mut st.int_freq, &names, "int_freq2");
            ui.add_space(8.0);
            let m = FREQS[st.int_freq].1;
            let nom = value(ui, pal, &st.int[0], env);
            let eff = value(ui, pal, &st.int[3], env);
            widgets::card(ui, pal, |ui| {
                ui.set_min_width(360.0);
                match (nom, eff) {
                    (Some(n), _) => {
                        big(ui, pal, "Effective annual rate", &pct(finance::effective_rate(n, m)));
                        widgets::row(ui, pal, "Rate per period", &pct(if m > 0.0 { n / m } else { n }));
                    }
                    (None, Some(e)) => {
                        big(ui, pal, "Nominal rate", &pct(finance::nominal_rate(e, m)));
                    }
                    _ => widgets::dim(ui, pal, "Enter either rate."),
                }
            });
        }
        _ => {
            widgets::dim(ui, pal, "Compound annual growth rate between two values.");
            labelled(ui, pal, "Starting value", &mut st.int[0], "e.g. 100000", "Value at the beginning.");
            labelled(ui, pal, "Ending value", &mut st.int[1], "e.g. 250000", "Value at the end.");
            labelled(ui, pal, "Years", &mut st.int[2], "e.g. 7", "Time between the two values, in years.");
            ui.add_space(8.0);
            let (Some(a), Some(b), Some(t)) = (value(ui, pal, &st.int[0], env), value(ui, pal, &st.int[1], env), value(ui, pal, &st.int[2], env)) else {
                widgets::dim(ui, pal, "Enter both values and the time.");
                return;
            };
            if a <= 0.0 || b <= 0.0 || t <= 0.0 {
                widgets::error(ui, pal, "Values and years must be positive.");
                return;
            }
            widgets::card(ui, pal, |ui| {
                ui.set_min_width(360.0);
                big(ui, pal, "CAGR", &pct(finance::cagr(a, b, t)));
                widgets::row(ui, pal, "Total growth", &pct((b / a - 1.0) * 100.0));
                widgets::row(ui, pal, "Multiple", &format!("{}×", fmt::real(b / a, Fmt::Fix(2))));
            });
        }
    }
}

// ---------------------------------------------------------------- Cash flow

fn cash_flow_ui(ui: &mut Ui, st: &mut FinState, env: &mut Env, pal: &Pal) {
    widgets::dim(ui, pal, "Net present value, internal rate of return and payback of a series of cash flows.");
    ui.add_space(6.0);
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            labelled(ui, pal, "Discount rate % / period", &mut st.cf_rate, "e.g. 10", "Required return per period (usually per year), in percent.");
            labelled(ui, pal, "Initial cash flow (CF0)", &mut st.cf0, "e.g. -500000", "The flow at time 0. An investment is negative.");
            labelled(ui, pal, "Reinvestment rate % (MIRR)", &mut st.cf_reinvest, "optional", "Rate at which positive flows are reinvested, for the modified IRR. Uses the discount rate when empty.");
        });
        ui.add_space(20.0);
        ui.vertical(|ui| {
            ui.label(RichText::new("Cash flows, one per period").size(13.5).color(pal.dim));
            egui::Frame::new()
                .fill(pal.surface)
                .stroke(egui::Stroke::new(1.0_f32, pal.line))
                .corner_radius(egui::CornerRadius::same(7))
                .inner_margin(egui::Margin::symmetric(9, 6))
                .show(ui, |ui| {
                    ui.add(
                        TextEdit::multiline(&mut st.cf_flows)
                            .frame(false)
                            .desired_width(180.0)
                            .desired_rows(5)
                            .hint_text(RichText::new("150000\n180000\n200000").color(pal.faint))
                            .font(theme::regular(14.0)),
                    )
                    .on_hover_text("One cash flow per line (or separated by commas), for periods 1, 2, 3 … Negative for money paid out.");
                });
        });
    });
    ui.add_space(8.0);
    let (Some(rate), Some(cf0)) = (value(ui, pal, &st.cf_rate, env), value(ui, pal, &st.cf0, env)) else {
        widgets::dim(ui, pal, "Enter the rate and the initial cash flow.");
        return;
    };
    let mut flows = Vec::new();
    for tok in st.cf_flows.split(|c: char| c == '\n' || c == ',' || c == ';').map(str::trim).filter(|s| !s.is_empty()) {
        match engine::field_value(tok, env) {
            Ok(Some(v)) => flows.push(v),
            _ => {
                widgets::error(ui, pal, format!("Cannot read “{}”", tok));
                return;
            }
        }
    }
    if flows.is_empty() {
        widgets::dim(ui, pal, "Enter at least one cash flow.");
        return;
    }
    let npv = finance::npv(rate, cf0, &flows);
    let irr = finance::irr(cf0, &flows);
    let reinvest = value(ui, pal, &st.cf_reinvest, env).unwrap_or(rate);
    widgets::card(ui, pal, |ui| {
        ui.set_min_width(360.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| big(ui, pal, "NPV", &money(npv)));
            ui.add_space(30.0);
            ui.vertical(|ui| big(ui, pal, "IRR", &irr.map(pct).unwrap_or_else(|| "none".into())));
        });
        let verdict = if npv > 0.0 { "NPV is positive: the project earns more than the discount rate." } else if npv < 0.0 { "NPV is negative: the project earns less than the discount rate." } else { "NPV is zero: the project exactly earns the discount rate." };
        ui.label(RichText::new(verdict).font(theme::medium(13.0)).color(if npv >= 0.0 { pal.accent } else { pal.dim }));
        ui.add_space(6.0);
        if let Some(m) = finance::mirr(cf0, &flows, rate, reinvest) {
            widgets::row(ui, pal, "Modified IRR", &pct(m));
        }
        let pv_in: f64 = npv - cf0;
        if cf0 < 0.0 {
            widgets::row(ui, pal, "Profitability index", &fmt::real(pv_in / -cf0, Fmt::Fix(3)));
        }
        let pb = |x: Option<f64>| x.map(|p| format!("{} periods", fmt::real(p, Fmt::Fix(2)))).unwrap_or_else(|| "never".into());
        widgets::row(ui, pal, "Payback period", &pb(finance::payback(0.0, cf0, &flows)));
        widgets::row(ui, pal, "Discounted payback", &pb(finance::payback(rate, cf0, &flows)));
        widgets::row(ui, pal, "Total inflow", &money(flows.iter().filter(|c| **c > 0.0).sum()));
    });
    ui.add_space(10.0);
    widgets::caption(ui, pal, "PRESENT VALUE OF EACH FLOW");
    let r = 1.0 + rate / 100.0;
    let mut cum = cf0;
    let mut data = vec![["0".to_string(), money(cf0), money(cf0), money(cf0)]];
    for (k, c) in flows.iter().enumerate() {
        let pv = c / r.powi(k as i32 + 1);
        cum += pv;
        data.push([(k + 1).to_string(), money(*c), money(pv), money(cum)]);
    }
    table(ui, pal, "cf_table", ["Period", "Cash flow", "Present value", "Cumulative PV"], &data);
}

// ---------------------------------------------------------------- Depreciation

fn depreciation_ui(ui: &mut Ui, st: &mut FinState, env: &mut Env, pal: &Pal) {
    widgets::dim(ui, pal, "How an asset loses value over its life, by the usual accounting methods.");
    ui.add_space(6.0);
    labelled(ui, pal, "Cost", &mut st.dep[0], "e.g. 1200000", "Purchase price of the asset.");
    labelled(ui, pal, "Salvage value", &mut st.dep[1], "e.g. 200000", "Expected value at the end of its life. 0 if none.");
    labelled(ui, pal, "Life (years)", &mut st.dep[2], "e.g. 5", "Useful life in whole years.");
    choice(ui, pal, "Method", &mut st.dep_method, &["Straight line", "Declining balance", "Double declining", "Sum of years' digits"], "dep_method");
    if st.dep_method == 1 {
        labelled(ui, pal, "Rate % of straight line", &mut st.dep[3], "e.g. 150", "Declining-balance factor: 150 for 1.5× the straight-line rate, 200 for double.");
    }
    ui.add_space(8.0);
    let (Some(cost), Some(salvage), Some(life)) = (value(ui, pal, &st.dep[0], env), value(ui, pal, &st.dep[1], env), value(ui, pal, &st.dep[2], env)) else {
        widgets::dim(ui, pal, "Enter the cost, salvage value and life.");
        return;
    };
    if cost <= 0.0 || salvage < 0.0 || salvage > cost || life < 1.0 || life > 100.0 {
        widgets::error(ui, pal, "Check the values: salvage must be between 0 and the cost, life between 1 and 100 years.");
        return;
    }
    let method = match st.dep_method {
        0 => DepMethod::StraightLine,
        1 => DepMethod::DecliningBalance(value(ui, pal, &st.dep[3], env).unwrap_or(150.0)),
        2 => DepMethod::DecliningBalance(200.0),
        _ => DepMethod::SumOfYears,
    };
    let rows = finance::depreciation(cost, salvage, life.round() as u32, method);
    widgets::card(ui, pal, |ui| {
        ui.set_min_width(360.0);
        big(ui, pal, "First-year depreciation", &money(rows[0].depreciation));
        widgets::row(ui, pal, "Total depreciation", &money(cost - salvage));
        if let DepMethod::DecliningBalance(p) = method {
            widgets::row(ui, pal, "Annual rate", &pct(p / life));
        }
    });
    ui.add_space(10.0);
    widgets::caption(ui, pal, "SCHEDULE");
    let data: Vec<[String; 4]> = rows.iter().map(|r| [r.year.to_string(), money(r.depreciation), money(r.accumulated), money(r.book_value)]).collect();
    table(ui, pal, "dep_table", ["Year", "Depreciation", "Accumulated", "Book value"], &data);
}

// ---------------------------------------------------------------- Business

fn business_ui(ui: &mut Ui, st: &mut FinState, env: &mut Env, pal: &Pal) {
    widgets::segmented(ui, pal, &mut st.biz_kind, &["Break-even", "Margin & markup", "Tax (GST / VAT)", "Percent change", "Discount"]);
    ui.add_space(10.0);
    let v = |ui: &mut Ui, s: &str, env: &mut Env| value(ui, pal, s, env);
    match st.biz_kind {
        0 => {
            labelled(ui, pal, "Fixed costs", &mut st.biz[0], "e.g. 300000", "Costs that do not change with the number sold: rent, salaries.");
            labelled(ui, pal, "Price per unit", &mut st.biz[1], "e.g. 250", "Selling price of one unit.");
            labelled(ui, pal, "Variable cost per unit", &mut st.biz[2], "e.g. 150", "Cost to make or buy one unit.");
            labelled(ui, pal, "Units sold (optional)", &mut st.biz[3], "e.g. 5000", "To see the profit at a given volume.");
            ui.add_space(8.0);
            let (Some(fixed), Some(price), Some(var)) = (v(ui, &st.biz[0], env), v(ui, &st.biz[1], env), v(ui, &st.biz[2], env)) else {
                widgets::dim(ui, pal, "Enter the fixed costs, price and variable cost.");
                return;
            };
            widgets::card(ui, pal, |ui| {
                ui.set_min_width(360.0);
                match finance::break_even(fixed, price, var) {
                    Some((units, revenue)) => {
                        big(ui, pal, "Break-even units", &fmt::real(units.ceil(), Fmt::Fix(0)));
                        widgets::row(ui, pal, "Break-even revenue", &money(revenue));
                        widgets::row(ui, pal, "Contribution per unit", &money(price - var));
                        widgets::row(ui, pal, "Contribution margin", &pct((price - var) / price * 100.0));
                        if let Some(q) = engine::field_value(&st.biz[3], env).ok().flatten() {
                            widgets::row(ui, pal, &format!("Profit at {} units", fmt::real(q, Fmt::Norm)), &money(q * (price - var) - fixed));
                            widgets::row(ui, pal, "Margin of safety", &pct((q - units) / q * 100.0));
                        }
                    }
                    None => widgets::error(ui, pal, "The price must be higher than the variable cost to ever break even."),
                }
            });
        }
        1 => {
            widgets::dim(ui, pal, "Enter any two of cost, price and margin to find the rest.");
            labelled(ui, pal, "Cost", &mut st.biz[0], "e.g. 800", "What you pay for the item.");
            labelled(ui, pal, "Selling price", &mut st.biz[1], "e.g. 1000", "What you sell it for.");
            labelled(ui, pal, "Margin %", &mut st.biz[2], "e.g. 20", "Profit as a share of the selling price.");
            ui.add_space(8.0);
            let cost = engine::field_value(&st.biz[0], env).ok().flatten();
            let price = engine::field_value(&st.biz[1], env).ok().flatten();
            let margin = engine::field_value(&st.biz[2], env).ok().flatten();
            let (c, p) = match (cost, price, margin) {
                (Some(c), Some(p), _) => (c, p),
                (Some(c), None, Some(m)) if m < 100.0 => (c, c / (1.0 - m / 100.0)),
                (None, Some(p), Some(m)) => (p * (1.0 - m / 100.0), p),
                _ => {
                    widgets::dim(ui, pal, "Enter two of the three values.");
                    return;
                }
            };
            widgets::card(ui, pal, |ui| {
                ui.set_min_width(360.0);
                big(ui, pal, "Profit per item", &money(p - c));
                widgets::row(ui, pal, "Cost", &money(c));
                widgets::row(ui, pal, "Selling price", &money(p));
                if p != 0.0 {
                    widgets::row(ui, pal, "Margin (of price)", &pct((p - c) / p * 100.0));
                }
                if c != 0.0 {
                    widgets::row(ui, pal, "Markup (on cost)", &pct((p - c) / c * 100.0));
                }
            });
        }
        2 => {
            labelled(ui, pal, "Amount", &mut st.biz[0], "e.g. 11800", "The amount before or after tax.");
            labelled(ui, pal, "Tax rate %", &mut st.biz[1], "e.g. 18", "GST, VAT or sales tax rate in percent.");
            ui.horizontal(|ui| {
                crate::widgets::row_label(ui, RichText::new("The amount is").size(13.5).color(pal.dim), 150.0, 34.0);
                let mut k = (st.biz[3] == "1") as usize;
                if widgets::segmented(ui, pal, &mut k, &["Before tax", "Tax included"]) {
                    st.biz[3] = k.to_string();
                }
            });
            ui.add_space(8.0);
            let (Some(a), Some(r)) = (v(ui, &st.biz[0], env), v(ui, &st.biz[1], env)) else {
                widgets::dim(ui, pal, "Enter the amount and the rate.");
                return;
            };
            let incl = st.biz[3] == "1";
            let (net, gross) = if incl { (a / (1.0 + r / 100.0), a) } else { (a, a * (1.0 + r / 100.0)) };
            widgets::card(ui, pal, |ui| {
                ui.set_min_width(360.0);
                big(ui, pal, if incl { "Amount before tax" } else { "Amount with tax" }, &money(if incl { net } else { gross }));
                widgets::row(ui, pal, "Tax", &money(gross - net));
                widgets::row(ui, pal, "Before tax", &money(net));
                widgets::row(ui, pal, "With tax", &money(gross));
                if r > 0.0 {
                    widgets::row(ui, pal, "Split in two (CGST + SGST)", &format!("{} each", money((gross - net) / 2.0)));
                }
            });
        }
        3 => {
            labelled(ui, pal, "From", &mut st.biz[0], "e.g. 120", "Old value.");
            labelled(ui, pal, "To", &mut st.biz[1], "e.g. 150", "New value.");
            ui.add_space(8.0);
            let (Some(a), Some(b)) = (v(ui, &st.biz[0], env), v(ui, &st.biz[1], env)) else {
                widgets::dim(ui, pal, "Enter both values.");
                return;
            };
            widgets::card(ui, pal, |ui| {
                ui.set_min_width(360.0);
                if a == 0.0 {
                    widgets::error(ui, pal, "Percent change from zero is undefined.");
                    return;
                }
                let ch = (b - a) / a.abs() * 100.0;
                big(ui, pal, if ch >= 0.0 { "Increase" } else { "Decrease" }, &pct(ch.abs()));
                widgets::row(ui, pal, "Difference", &money(b - a));
                widgets::row(ui, pal, "Ratio", &format!("{}×", fmt::real(b / a, Fmt::Fix(4))));
                widgets::row(ui, pal, "Change needed to go back", &pct(((a - b) / b.abs() * 100.0).abs()));
            });
        }
        _ => {
            labelled(ui, pal, "Original price", &mut st.biz[0], "e.g. 2499", "Price before the discount.");
            labelled(ui, pal, "Discount %", &mut st.biz[1], "e.g. 30", "Discount in percent. Add a second one like 30+10 for stacked discounts.");
            ui.add_space(8.0);
            let (Some(p), Some(d)) = (v(ui, &st.biz[0], env), v(ui, &st.biz[1], env)) else {
                widgets::dim(ui, pal, "Enter the price and the discount.");
                return;
            };
            // "30+10" is evaluated as 40 by the engine; treat stacked discounts via the raw text.
            let parts: Vec<f64> = st.biz[1]
                .split('+')
                .filter_map(|s| engine::field_value(s.trim(), env).ok().flatten())
                .collect();
            let factor: f64 = if parts.len() > 1 { parts.iter().map(|x| 1.0 - x / 100.0).product() } else { 1.0 - d / 100.0 };
            let final_price = p * factor;
            widgets::card(ui, pal, |ui| {
                ui.set_min_width(360.0);
                big(ui, pal, "You pay", &money(final_price));
                widgets::row(ui, pal, "You save", &money(p - final_price));
                widgets::row(ui, pal, "Effective discount", &pct((1.0 - factor) * 100.0));
            });
        }
    }
}
