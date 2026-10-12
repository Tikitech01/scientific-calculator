//! Financial maths: time value of money, loans, interest, cash flows, depreciation and
//! everyday business figures. Rates are given in percent per year unless stated otherwise.

/// Which of the five time-value-of-money quantities to solve for.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TvmVar {
    N,
    Rate,
    Pv,
    Pmt,
    Fv,
}

/// The time-value-of-money register set, as on a financial calculator.
/// Money paid out is negative, money received is positive.
#[derive(Clone, Copy, Debug)]
pub struct Tvm {
    /// Number of payments.
    pub n: f64,
    /// Nominal annual rate in percent.
    pub rate: f64,
    pub pv: f64,
    pub pmt: f64,
    pub fv: f64,
    /// Payments per year.
    pub py: f64,
    /// Compounding periods per year.
    pub cy: f64,
    /// Payments at the beginning of each period (annuity due) instead of the end.
    pub begin: bool,
}

impl Tvm {
    /// Interest rate per payment period, as a fraction.
    fn period_rate(&self) -> f64 {
        let r = self.rate / 100.0;
        if self.cy == self.py {
            r / self.py
        } else {
            (1.0 + r / self.cy).powf(self.cy / self.py) - 1.0
        }
    }

    /// Left-hand side of the TVM equation for a period rate `i`; zero when the registers agree.
    fn balance(&self, i: f64) -> f64 {
        let g = if self.begin { 1.0 + i } else { 1.0 };
        if i.abs() < 1e-12 {
            self.pv + self.pmt * self.n + self.fv
        } else {
            let d = (1.0 + i).powf(-self.n);
            self.pv + self.pmt * g * (1.0 - d) / i + self.fv * d
        }
    }

    /// Solve for one register from the other four.
    pub fn solve(&self, var: TvmVar) -> Result<f64, String> {
        if self.py <= 0.0 || self.cy <= 0.0 {
            return Err("P/Y and C/Y must be positive".into());
        }
        let i = self.period_rate();
        let g = if self.begin { 1.0 + i } else { 1.0 };
        match var {
            TvmVar::Pv => {
                if i.abs() < 1e-12 {
                    Ok(-(self.pmt * self.n + self.fv))
                } else {
                    let d = (1.0 + i).powf(-self.n);
                    Ok(-(self.pmt * g * (1.0 - d) / i + self.fv * d))
                }
            }
            TvmVar::Fv => {
                if i.abs() < 1e-12 {
                    Ok(-(self.pv + self.pmt * self.n))
                } else {
                    let a = (1.0 + i).powf(self.n);
                    Ok(-(self.pv * a + self.pmt * g * (a - 1.0) / i))
                }
            }
            TvmVar::Pmt => {
                if self.n <= 0.0 {
                    return Err("N must be positive".into());
                }
                if i.abs() < 1e-12 {
                    Ok(-(self.pv + self.fv) / self.n)
                } else {
                    let d = (1.0 + i).powf(-self.n);
                    Ok(-(self.pv + self.fv * d) * i / (g * (1.0 - d)))
                }
            }
            TvmVar::N => {
                if i.abs() < 1e-12 {
                    if self.pmt == 0.0 {
                        return Err("With no interest and no payment, N is undefined".into());
                    }
                    return Ok(-(self.pv + self.fv) / self.pmt);
                }
                let a = self.pmt * g / i;
                let num = (a - self.fv) / (a + self.pv);
                if num <= 0.0 || !num.is_finite() {
                    return Err("No solution: the payment never pays this off. Check the signs (money paid is negative).".into());
                }
                Ok(num.ln() / (1.0 + i).ln())
            }
            TvmVar::Rate => {
                if self.n <= 0.0 {
                    return Err("N must be positive".into());
                }
                let f = |i: f64| self.balance(i);
                // Look for a sign change between a tiny negative rate and a very large one.
                let lo0 = -0.99;
                let mut lo = lo0;
                let mut hi = 1e-6;
                let mut found = false;
                let mut x = hi;
                while x < 10.0 {
                    if f(lo).signum() != f(x).signum() && f(lo).is_finite() && f(x).is_finite() {
                        hi = x;
                        found = true;
                        break;
                    }
                    lo = x;
                    x *= 1.5;
                }
                if !found {
                    return Err("No rate fits these values. Check the signs (money paid is negative, money received is positive).".into());
                }
                for _ in 0..200 {
                    let mid = 0.5 * (lo + hi);
                    if f(lo).signum() == f(mid).signum() {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                let i = 0.5 * (lo + hi);
                // Back to a nominal annual rate in percent.
                let r = if self.cy == self.py { i * self.py } else { ((1.0 + i).powf(self.py / self.cy) - 1.0) * self.cy };
                Ok(r * 100.0)
            }
        }
    }
}

/// One line of a loan schedule.
#[derive(Clone, Debug, PartialEq)]
pub struct LoanRow {
    pub period: u32,
    pub payment: f64,
    pub principal: f64,
    pub interest: f64,
    pub balance: f64,
}

/// Fixed instalment (EMI) for a loan: `rate` is the annual percentage, `months` the term.
pub fn emi(principal: f64, rate: f64, months: f64) -> f64 {
    let i = rate / 100.0 / 12.0;
    if months <= 0.0 {
        return f64::NAN;
    }
    if i.abs() < 1e-12 {
        principal / months
    } else {
        principal * i / (1.0 - (1.0 + i).powf(-months))
    }
}

/// Month-by-month schedule. `extra` is an optional additional principal payment each month.
pub fn amortization(principal: f64, rate: f64, months: u32, extra: f64) -> Vec<LoanRow> {
    let i = rate / 100.0 / 12.0;
    let pay = emi(principal, rate, months as f64);
    let mut rows = Vec::new();
    let mut balance = principal;
    for m in 1..=months {
        if balance <= 0.005 {
            break;
        }
        let interest = balance * i;
        let mut principal_part = pay - interest + extra;
        if principal_part > balance {
            principal_part = balance;
        }
        balance -= principal_part;
        rows.push(LoanRow { period: m, payment: principal_part + interest, principal: principal_part, interest, balance: balance.max(0.0) });
    }
    rows
}

/// Group a monthly schedule into years.
pub fn yearly(rows: &[LoanRow]) -> Vec<LoanRow> {
    let mut out: Vec<LoanRow> = Vec::new();
    for r in rows {
        let year = (r.period - 1) / 12 + 1;
        match out.last_mut() {
            Some(last) if last.period == year => {
                last.payment += r.payment;
                last.principal += r.principal;
                last.interest += r.interest;
                last.balance = r.balance;
            }
            _ => out.push(LoanRow { period: year, ..r.clone() }),
        }
    }
    out
}

/// Amount after compound interest. `freq` compounding periods per year; 0 means continuous.
pub fn compound(principal: f64, rate: f64, years: f64, freq: f64) -> f64 {
    let r = rate / 100.0;
    if freq <= 0.0 {
        principal * (r * years).exp()
    } else {
        principal * (1.0 + r / freq).powf(freq * years)
    }
}

pub fn simple(principal: f64, rate: f64, years: f64) -> f64 {
    principal * (1.0 + rate / 100.0 * years)
}

/// Effective annual rate (percent) of a nominal rate compounded `m` times a year.
pub fn effective_rate(nominal: f64, m: f64) -> f64 {
    if m <= 0.0 {
        ((nominal / 100.0).exp() - 1.0) * 100.0
    } else {
        ((1.0 + nominal / 100.0 / m).powf(m) - 1.0) * 100.0
    }
}

/// Nominal rate (percent) that gives an effective annual rate with `m` compoundings a year.
pub fn nominal_rate(effective: f64, m: f64) -> f64 {
    if m <= 0.0 {
        (1.0 + effective / 100.0).ln() * 100.0
    } else {
        ((1.0 + effective / 100.0).powf(1.0 / m) - 1.0) * m * 100.0
    }
}

/// Compound annual growth rate in percent.
pub fn cagr(start: f64, end: f64, years: f64) -> f64 {
    ((end / start).powf(1.0 / years) - 1.0) * 100.0
}

/// Years for money to double at an annual rate (percent), compounded yearly.
pub fn doubling_time(rate: f64) -> f64 {
    2f64.ln() / (1.0 + rate / 100.0).ln()
}

/// Net present value: `rate` in percent per period, `cf0` the initial flow (usually negative),
/// `flows` the flows at the end of periods 1, 2, 3 …
pub fn npv(rate: f64, cf0: f64, flows: &[f64]) -> f64 {
    let r = 1.0 + rate / 100.0;
    cf0 + flows.iter().enumerate().map(|(k, c)| c / r.powi(k as i32 + 1)).sum::<f64>()
}

/// Internal rate of return in percent per period, if one exists between −99% and 10 000%.
pub fn irr(cf0: f64, flows: &[f64]) -> Option<f64> {
    let f = |r: f64| npv(r, cf0, flows);
    let mut lo = -99.0;
    let mut step = 0.5;
    let mut x = lo + step;
    let mut hi = None;
    while x <= 10_000.0 {
        if f(lo).is_finite() && f(x).is_finite() && f(lo).signum() != f(x).signum() {
            hi = Some(x);
            break;
        }
        lo = x;
        step *= 1.2;
        x += step;
    }
    let mut hi = hi?;
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if f(lo).signum() == f(mid).signum() {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(0.5 * (lo + hi))
}

/// Modified IRR in percent per period.
pub fn mirr(cf0: f64, flows: &[f64], finance_rate: f64, reinvest_rate: f64) -> Option<f64> {
    let n = flows.len() as f64;
    if n == 0.0 {
        return None;
    }
    let fr = 1.0 + finance_rate / 100.0;
    let rr = 1.0 + reinvest_rate / 100.0;
    let mut pv_neg = if cf0 < 0.0 { cf0 } else { 0.0 };
    let mut fv_pos = if cf0 > 0.0 { cf0 * rr.powf(n) } else { 0.0 };
    for (k, c) in flows.iter().enumerate() {
        let t = k as f64 + 1.0;
        if *c < 0.0 {
            pv_neg += c / fr.powf(t);
        } else {
            fv_pos += c * rr.powf(n - t);
        }
    }
    if pv_neg >= 0.0 || fv_pos <= 0.0 {
        return None;
    }
    Some(((fv_pos / -pv_neg).powf(1.0 / n) - 1.0) * 100.0)
}

/// Periods until the cumulative cash flow turns positive (fractional), discounted at `rate`
/// (percent; 0 gives the simple payback).
pub fn payback(rate: f64, cf0: f64, flows: &[f64]) -> Option<f64> {
    let r = 1.0 + rate / 100.0;
    let mut cum = cf0;
    if cum >= 0.0 {
        return Some(0.0);
    }
    for (k, c) in flows.iter().enumerate() {
        let d = c / r.powi(k as i32 + 1);
        if cum + d >= 0.0 {
            return Some(k as f64 + if d > 0.0 { -cum / d } else { 0.0 });
        }
        cum += d;
    }
    None
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DepMethod {
    StraightLine,
    /// Declining balance at a fixed percentage of the straight-line rate (200% = double declining).
    DecliningBalance(f64),
    SumOfYears,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DepRow {
    pub year: u32,
    pub depreciation: f64,
    pub accumulated: f64,
    pub book_value: f64,
}

/// Depreciation schedule over `life` whole years. Book value never falls below the salvage value.
pub fn depreciation(cost: f64, salvage: f64, life: u32, method: DepMethod) -> Vec<DepRow> {
    let mut rows = Vec::new();
    let mut book = cost;
    let base = cost - salvage;
    let n = life as f64;
    for y in 1..=life {
        let mut d = match method {
            DepMethod::StraightLine => base / n,
            DepMethod::SumOfYears => base * (n - y as f64 + 1.0) / (n * (n + 1.0) / 2.0),
            DepMethod::DecliningBalance(pct) => book * (pct / 100.0) / n,
        };
        if matches!(method, DepMethod::DecliningBalance(_)) && y == life {
            d = book - salvage; // write the remainder off in the last year
        }
        d = d.max(0.0).min(book - salvage).max(0.0);
        book -= d;
        rows.push(DepRow { year: y, depreciation: d, accumulated: cost - book, book_value: book });
    }
    rows
}

/// Units and revenue needed to cover fixed costs.
pub fn break_even(fixed: f64, price: f64, variable: f64) -> Option<(f64, f64)> {
    let margin = price - variable;
    if margin <= 0.0 {
        return None;
    }
    let units = fixed / margin;
    Some((units, units * price))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() < tol, "{} vs {}", a, b);
    }

    #[test]
    fn tvm_round_trip() {
        // 200 000 at 9 % for 20 years, monthly: EMI 1 799.45
        let t = Tvm { n: 240.0, rate: 9.0, pv: 200_000.0, pmt: 0.0, fv: 0.0, py: 12.0, cy: 12.0, begin: false };
        let pmt = t.solve(TvmVar::Pmt).unwrap();
        close(pmt, -1799.45, 0.01);
        let t2 = Tvm { pmt, ..t };
        close(t2.solve(TvmVar::N).unwrap(), 240.0, 1e-6);
        close(t2.solve(TvmVar::Rate).unwrap(), 9.0, 1e-6);
        close(t2.solve(TvmVar::Pv).unwrap(), 200_000.0, 1e-4);
        close(t2.solve(TvmVar::Fv).unwrap(), 0.0, 1e-4);
    }

    #[test]
    fn tvm_savings_begin() {
        // 500 a month for 10 years at 6 %, paid at the start of each month.
        let t = Tvm { n: 120.0, rate: 6.0, pv: 0.0, pmt: -500.0, fv: 0.0, py: 12.0, cy: 12.0, begin: true };
        close(t.solve(TvmVar::Fv).unwrap(), 82_349.37, 0.01);
    }

    #[test]
    fn loan_schedule() {
        close(emi(1_000_000.0, 8.5, 240.0), 8678.23, 0.01);
        let rows = amortization(100_000.0, 12.0, 12, 0.0);
        assert_eq!(rows.len(), 12);
        close(rows[0].interest, 1000.0, 1e-9);
        close(rows.last().unwrap().balance, 0.0, 1e-6);
        let total: f64 = rows.iter().map(|r| r.interest).sum();
        close(total, 6618.55, 0.01);
        assert_eq!(yearly(&rows).len(), 1);
    }

    #[test]
    fn interest() {
        close(compound(1000.0, 5.0, 10.0, 1.0), 1628.89, 0.01);
        close(compound(1000.0, 5.0, 10.0, 12.0), 1647.01, 0.01);
        close(compound(1000.0, 5.0, 10.0, 0.0), 1648.72, 0.01);
        close(simple(1000.0, 5.0, 10.0), 1500.0, 1e-9);
        close(effective_rate(12.0, 12.0), 12.6825, 1e-4);
        close(nominal_rate(12.6825, 12.0), 12.0, 1e-3);
        close(cagr(100.0, 200.0, 10.0), 7.1773, 1e-4);
        close(doubling_time(7.1773), 10.0, 1e-3);
    }

    #[test]
    fn cash_flows() {
        let flows = [3000.0, 4200.0, 6800.0];
        close(npv(10.0, -10_000.0, &flows), 1307.29, 0.01);
        close(irr(-10_000.0, &flows).unwrap(), 16.3406, 0.001);
        close(payback(0.0, -10_000.0, &flows).unwrap(), 2.41, 0.01);
        close(mirr(-10_000.0, &flows, 10.0, 12.0).unwrap(), 15.147, 0.001);
        assert!(irr(100.0, &[10.0]).is_none());
        close(irr(-100.0, &[10.0]).unwrap(), -90.0, 1e-6);
    }

    #[test]
    fn depreciation_methods() {
        let sl = depreciation(10_000.0, 1000.0, 5, DepMethod::StraightLine);
        close(sl[0].depreciation, 1800.0, 1e-9);
        close(sl[4].book_value, 1000.0, 1e-9);
        let syd = depreciation(10_000.0, 1000.0, 5, DepMethod::SumOfYears);
        close(syd[0].depreciation, 3000.0, 1e-9);
        close(syd[4].book_value, 1000.0, 1e-9);
        let ddb = depreciation(10_000.0, 1000.0, 5, DepMethod::DecliningBalance(200.0));
        close(ddb[0].depreciation, 4000.0, 1e-9);
        close(ddb[4].book_value, 1000.0, 1e-9);
    }

    #[test]
    fn business() {
        let (u, r) = break_even(50_000.0, 25.0, 15.0).unwrap();
        close(u, 5000.0, 1e-9);
        close(r, 125_000.0, 1e-9);
        assert!(break_even(1.0, 1.0, 2.0).is_none());
    }
}
