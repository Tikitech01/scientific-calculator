//! Hypothesis tests and the t, χ² and F distributions they rely on.

use crate::stats::{inv_norm, norm_cdf};

// ---------------------------------------------------------------------------
// Special functions

fn ln_gamma(x: f64) -> f64 {
    libm::lgamma(x)
}

/// Regularized lower incomplete gamma function P(a, x).
pub fn gamma_p(a: f64, x: f64) -> f64 {
    if x <= 0.0 || a <= 0.0 {
        return 0.0;
    }
    let front = (-x + a * x.ln() - ln_gamma(a)).exp();
    if x < a + 1.0 {
        // Series.
        let (mut ap, mut sum) = (a, 1.0 / a);
        let mut del = sum;
        for _ in 0..1000 {
            ap += 1.0;
            del *= x / ap;
            sum += del;
            if del.abs() < sum.abs() * 1e-16 {
                break;
            }
        }
        (sum * front).clamp(0.0, 1.0)
    } else {
        // Continued fraction for Q(a, x), Lentz's method.
        let tiny = 1e-300;
        let mut b = x + 1.0 - a;
        let mut c = 1.0 / tiny;
        let mut d = 1.0 / b;
        let mut h = d;
        for i in 1..1000 {
            let an = -(i as f64) * (i as f64 - a);
            b += 2.0;
            d = an * d + b;
            if d.abs() < tiny {
                d = tiny;
            }
            c = b + an / c;
            if c.abs() < tiny {
                c = tiny;
            }
            d = 1.0 / d;
            let del = d * c;
            h *= del;
            if (del - 1.0).abs() < 1e-16 {
                break;
            }
        }
        (1.0 - front * h).clamp(0.0, 1.0)
    }
}

fn beta_cf(a: f64, b: f64, x: f64) -> f64 {
    let tiny = 1e-300;
    let (qab, qap, qam) = (a + b, a + 1.0, a - 1.0);
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < tiny {
        d = tiny;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..1000 {
        let m = m as f64;
        let m2 = 2.0 * m;
        let aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < tiny {
            d = tiny;
        }
        c = 1.0 + aa / c;
        if c.abs() < tiny {
            c = tiny;
        }
        d = 1.0 / d;
        h *= d * c;
        let aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < tiny {
            d = tiny;
        }
        c = 1.0 + aa / c;
        if c.abs() < tiny {
            c = tiny;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < 1e-16 {
            break;
        }
    }
    h
}

/// Regularized incomplete beta function I_x(a, b).
pub fn inc_beta(a: f64, b: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let bt = (ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (1.0 - x).ln()).exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        bt * beta_cf(a, b, x) / a
    } else {
        1.0 - bt * beta_cf(b, a, 1.0 - x) / b
    }
}

// ---------------------------------------------------------------------------
// Distributions

pub fn t_pdf(t: f64, v: f64) -> f64 {
    (ln_gamma((v + 1.0) / 2.0) - ln_gamma(v / 2.0)).exp() / (v * std::f64::consts::PI).sqrt()
        * (1.0 + t * t / v).powf(-(v + 1.0) / 2.0)
}

pub fn t_cdf(t: f64, v: f64) -> f64 {
    if t.is_infinite() {
        return if t > 0.0 { 1.0 } else { 0.0 };
    }
    let tail = 0.5 * inc_beta(v / 2.0, 0.5, v / (v + t * t));
    if t > 0.0 {
        1.0 - tail
    } else {
        tail
    }
}

pub fn chi2_pdf(x: f64, k: f64) -> f64 {
    if x < 0.0 {
        return 0.0;
    }
    if x == 0.0 {
        return if k == 2.0 { 0.5 } else if k < 2.0 { f64::INFINITY } else { 0.0 };
    }
    ((k / 2.0 - 1.0) * x.ln() - x / 2.0 - (k / 2.0) * 2f64.ln() - ln_gamma(k / 2.0)).exp()
}

pub fn chi2_cdf(x: f64, k: f64) -> f64 {
    if x.is_infinite() {
        return if x > 0.0 { 1.0 } else { 0.0 };
    }
    gamma_p(k / 2.0, x / 2.0)
}

pub fn f_pdf(x: f64, d1: f64, d2: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    let ln = 0.5 * (d1 * (d1 * x).ln() + d2 * d2.ln() - (d1 + d2) * (d1 * x + d2).ln())
        - x.ln()
        - (ln_gamma(d1 / 2.0) + ln_gamma(d2 / 2.0) - ln_gamma((d1 + d2) / 2.0));
    ln.exp()
}

pub fn f_cdf(x: f64, d1: f64, d2: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x.is_infinite() {
        return 1.0;
    }
    inc_beta(d1 / 2.0, d2 / 2.0, d1 * x / (d1 * x + d2))
}

/// Invert a continuous, increasing cdf by bisection.
fn invert(cdf: impl Fn(f64) -> f64, p: f64, mut lo: f64, mut hi: f64) -> f64 {
    // Widen the bracket until it contains the answer.
    let mut guard = 0;
    while cdf(hi) < p && guard < 200 {
        hi = hi * 2.0 + 1.0;
        guard += 1;
    }
    guard = 0;
    while cdf(lo) > p && guard < 200 {
        lo = if lo < 0.0 { lo * 2.0 - 1.0 } else { lo / 2.0 };
        guard += 1;
    }
    for _ in 0..300 {
        let mid = 0.5 * (lo + hi);
        if cdf(mid) < p {
            lo = mid;
        } else {
            hi = mid;
        }
        if (hi - lo).abs() <= 1e-14 * (1.0 + mid.abs()) {
            break;
        }
    }
    0.5 * (lo + hi)
}

pub fn inv_t(p: f64, v: f64) -> f64 {
    invert(|t| t_cdf(t, v), p, -10.0, 10.0)
}

pub fn inv_chi2(p: f64, k: f64) -> f64 {
    invert(|x| chi2_cdf(x, k), p, 0.0, k.max(1.0) * 4.0)
}

pub fn inv_f(p: f64, d1: f64, d2: f64) -> f64 {
    invert(|x| f_cdf(x, d1, d2), p, 0.0, 10.0)
}

// ---------------------------------------------------------------------------
// Tests

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tail {
    /// H₁: parameter ≠ value
    Two,
    /// H₁: parameter < value
    Less,
    /// H₁: parameter > value
    Greater,
}

/// One line of a result: label and value (text so degrees of freedom and intervals read naturally).
pub type Row = (String, f64);

pub struct Outcome {
    pub statistic: (&'static str, f64),
    pub p: f64,
    pub rows: Vec<Row>,
    /// Lower and upper confidence limits, with their label.
    pub ci: Option<(String, f64, f64)>,
    /// Critical value(s) for the chosen α, as text.
    pub critical: Option<String>,
    pub note: Option<String>,
}

fn p_from(cdf_at_stat: f64, tail: Tail) -> f64 {
    match tail {
        Tail::Two => (2.0 * cdf_at_stat.min(1.0 - cdf_at_stat)).min(1.0),
        Tail::Less => cdf_at_stat,
        Tail::Greater => 1.0 - cdf_at_stat,
    }
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

fn var(v: &[f64]) -> f64 {
    let m = mean(v);
    v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (v.len() as f64 - 1.0)
}

fn t_critical(alpha: f64, df: f64, tail: Tail, name: &str) -> String {
    match tail {
        Tail::Two => format!("{} = ±{}", name, crate::fmt::short(inv_t(1.0 - alpha / 2.0, df), 6)),
        Tail::Less => format!("{} = {}", name, crate::fmt::short(inv_t(alpha, df), 6)),
        Tail::Greater => format!("{} = {}", name, crate::fmt::short(inv_t(1.0 - alpha, df), 6)),
    }
}

fn z_critical(alpha: f64, tail: Tail) -> String {
    match tail {
        Tail::Two => format!("z = ±{}", crate::fmt::short(inv_norm(1.0 - alpha / 2.0), 6)),
        Tail::Less => format!("z = {}", crate::fmt::short(inv_norm(alpha), 6)),
        Tail::Greater => format!("z = {}", crate::fmt::short(inv_norm(1.0 - alpha), 6)),
    }
}

fn need(v: &[f64], n: usize, what: &str) -> Result<(), String> {
    if v.len() < n {
        Err(format!("{} needs at least {} values", what, n))
    } else {
        Ok(())
    }
}

pub fn t_one_sample(x: &[f64], mu0: f64, tail: Tail, alpha: f64) -> Result<Outcome, String> {
    need(x, 2, "The sample")?;
    let n = x.len() as f64;
    let (m, s) = (mean(x), var(x).sqrt());
    if s == 0.0 {
        return Err("All values are equal, so the standard deviation is 0".into());
    }
    let se = s / n.sqrt();
    let t = (m - mu0) / se;
    let df = n - 1.0;
    let tc = inv_t(1.0 - alpha / 2.0, df);
    Ok(Outcome {
        statistic: ("t", t),
        p: p_from(t_cdf(t, df), tail),
        rows: vec![
            ("n".into(), n),
            ("Mean x̄".into(), m),
            ("Standard deviation s".into(), s),
            ("Standard error".into(), se),
            ("Degrees of freedom".into(), df),
        ],
        ci: Some((format!("{}% confidence interval for μ", (1.0 - alpha) * 100.0), m - tc * se, m + tc * se)),
        critical: Some(t_critical(alpha, df, tail, "t")),
        note: None,
    })
}

pub fn t_two_sample(x: &[f64], y: &[f64], pooled: bool, tail: Tail, alpha: f64) -> Result<Outcome, String> {
    need(x, 2, "Sample X")?;
    need(y, 2, "Sample Y")?;
    let (n1, n2) = (x.len() as f64, y.len() as f64);
    let (m1, m2, v1, v2) = (mean(x), mean(y), var(x), var(y));
    let (se, df) = if pooled {
        let sp2 = ((n1 - 1.0) * v1 + (n2 - 1.0) * v2) / (n1 + n2 - 2.0);
        ((sp2 * (1.0 / n1 + 1.0 / n2)).sqrt(), n1 + n2 - 2.0)
    } else {
        let (a, b) = (v1 / n1, v2 / n2);
        ((a + b).sqrt(), (a + b).powi(2) / (a * a / (n1 - 1.0) + b * b / (n2 - 1.0)))
    };
    if se == 0.0 {
        return Err("Both samples have no spread, so the test cannot be done".into());
    }
    let d = m1 - m2;
    let t = d / se;
    let tc = inv_t(1.0 - alpha / 2.0, df);
    Ok(Outcome {
        statistic: ("t", t),
        p: p_from(t_cdf(t, df), tail),
        rows: vec![
            ("n₁ (X)".into(), n1),
            ("n₂ (Y)".into(), n2),
            ("Mean x̄₁".into(), m1),
            ("Mean x̄₂".into(), m2),
            ("Std deviation s₁".into(), v1.sqrt()),
            ("Std deviation s₂".into(), v2.sqrt()),
            ("Difference x̄₁ − x̄₂".into(), d),
            ("Standard error".into(), se),
            ("Degrees of freedom".into(), df),
        ],
        ci: Some((format!("{}% confidence interval for μ₁ − μ₂", (1.0 - alpha) * 100.0), d - tc * se, d + tc * se)),
        critical: Some(t_critical(alpha, df, tail, "t")),
        note: Some(if pooled { "Pooled variance (equal variances assumed)".into() } else { "Welch's test (variances not assumed equal)".into() }),
    })
}

pub fn t_paired(x: &[f64], y: &[f64], tail: Tail, alpha: f64) -> Result<Outcome, String> {
    if x.len() != y.len() {
        return Err("A paired test needs the same number of X and Y values".into());
    }
    let diffs: Vec<f64> = x.iter().zip(y).map(|(a, b)| a - b).collect();
    let mut o = t_one_sample(&diffs, 0.0, tail, alpha)?;
    o.rows[1].0 = "Mean difference d̄".into();
    o.rows[2].0 = "Std deviation of differences".into();
    if let Some(ci) = &mut o.ci {
        ci.0 = format!("{}% confidence interval for μ_d", (1.0 - alpha) * 100.0);
    }
    o.note = Some("Differences are X − Y, row by row".into());
    Ok(o)
}

pub fn z_one_mean(x: &[f64], mu0: f64, sigma: f64, tail: Tail, alpha: f64) -> Result<Outcome, String> {
    need(x, 1, "The sample")?;
    if sigma <= 0.0 {
        return Err("σ must be positive".into());
    }
    let n = x.len() as f64;
    let m = mean(x);
    let se = sigma / n.sqrt();
    let z = (m - mu0) / se;
    let zc = inv_norm(1.0 - alpha / 2.0);
    Ok(Outcome {
        statistic: ("z", z),
        p: p_from(norm_cdf(z, 0.0, 1.0), tail),
        rows: vec![("n".into(), n), ("Mean x̄".into(), m), ("Standard error σ/√n".into(), se)],
        ci: Some((format!("{}% confidence interval for μ", (1.0 - alpha) * 100.0), m - zc * se, m + zc * se)),
        critical: Some(z_critical(alpha, tail)),
        note: None,
    })
}

pub fn z_one_prop(successes: f64, n: f64, p0: f64, tail: Tail, alpha: f64) -> Result<Outcome, String> {
    if n <= 0.0 || successes < 0.0 || successes > n {
        return Err("Successes must be between 0 and n".into());
    }
    if !(p0 > 0.0 && p0 < 1.0) {
        return Err("p₀ must be between 0 and 1".into());
    }
    let ph = successes / n;
    let z = (ph - p0) / (p0 * (1.0 - p0) / n).sqrt();
    let zc = inv_norm(1.0 - alpha / 2.0);
    let se = (ph * (1.0 - ph) / n).sqrt();
    Ok(Outcome {
        statistic: ("z", z),
        p: p_from(norm_cdf(z, 0.0, 1.0), tail),
        rows: vec![("Sample proportion p̂".into(), ph), ("n".into(), n)],
        ci: Some((format!("{}% confidence interval for p", (1.0 - alpha) * 100.0), ph - zc * se, ph + zc * se)),
        critical: Some(z_critical(alpha, tail)),
        note: None,
    })
}

pub fn z_two_prop(x1: f64, n1: f64, x2: f64, n2: f64, tail: Tail, alpha: f64) -> Result<Outcome, String> {
    if n1 <= 0.0 || n2 <= 0.0 || x1 < 0.0 || x2 < 0.0 || x1 > n1 || x2 > n2 {
        return Err("Successes must be between 0 and n for both groups".into());
    }
    let (p1, p2) = (x1 / n1, x2 / n2);
    let pp = (x1 + x2) / (n1 + n2);
    let se0 = (pp * (1.0 - pp) * (1.0 / n1 + 1.0 / n2)).sqrt();
    if se0 == 0.0 {
        return Err("The pooled proportion is 0 or 1, so the test cannot be done".into());
    }
    let z = (p1 - p2) / se0;
    let se = (p1 * (1.0 - p1) / n1 + p2 * (1.0 - p2) / n2).sqrt();
    let zc = inv_norm(1.0 - alpha / 2.0);
    Ok(Outcome {
        statistic: ("z", z),
        p: p_from(norm_cdf(z, 0.0, 1.0), tail),
        rows: vec![("p̂₁".into(), p1), ("p̂₂".into(), p2), ("Pooled p̂".into(), pp), ("Difference p̂₁ − p̂₂".into(), p1 - p2)],
        ci: Some((format!("{}% confidence interval for p₁ − p₂", (1.0 - alpha) * 100.0), p1 - p2 - zc * se, p1 - p2 + zc * se)),
        critical: Some(z_critical(alpha, tail)),
        note: None,
    })
}

pub fn f_test(x: &[f64], y: &[f64], tail: Tail, alpha: f64) -> Result<Outcome, String> {
    need(x, 2, "Sample X")?;
    need(y, 2, "Sample Y")?;
    let (v1, v2) = (var(x), var(y));
    if v2 == 0.0 {
        return Err("Sample Y has no spread, so the ratio is undefined".into());
    }
    let (d1, d2) = (x.len() as f64 - 1.0, y.len() as f64 - 1.0);
    let f = v1 / v2;
    let critical = match tail {
        Tail::Two => format!(
            "F = {} and {}",
            crate::fmt::short(inv_f(alpha / 2.0, d1, d2), 6),
            crate::fmt::short(inv_f(1.0 - alpha / 2.0, d1, d2), 6)
        ),
        Tail::Less => format!("F = {}", crate::fmt::short(inv_f(alpha, d1, d2), 6)),
        Tail::Greater => format!("F = {}", crate::fmt::short(inv_f(1.0 - alpha, d1, d2), 6)),
    };
    Ok(Outcome {
        statistic: ("F", f),
        p: p_from(f_cdf(f, d1, d2), tail),
        rows: vec![
            ("Variance s₁² (X)".into(), v1),
            ("Variance s₂² (Y)".into(), v2),
            ("df₁ (numerator)".into(), d1),
            ("df₂ (denominator)".into(), d2),
        ],
        ci: Some((
            format!("{}% confidence interval for σ₁²/σ₂²", (1.0 - alpha) * 100.0),
            f / inv_f(1.0 - alpha / 2.0, d1, d2),
            f / inv_f(alpha / 2.0, d1, d2),
        )),
        critical: Some(critical),
        note: Some("H₀: the two populations have equal variances".into()),
    })
}

pub fn chi2_gof(observed: &[f64], expected: Option<&[f64]>, alpha: f64) -> Result<Outcome, String> {
    need(observed, 2, "Observed counts")?;
    let total: f64 = observed.iter().sum();
    let exp: Vec<f64> = match expected {
        Some(e) if !e.is_empty() => {
            if e.len() != observed.len() {
                return Err("Observed (X) and expected (Y) need the same number of rows".into());
            }
            // Expected may be counts or proportions: scale them to the observed total.
            let es: f64 = e.iter().sum();
            if es <= 0.0 {
                return Err("Expected values must be positive".into());
            }
            e.iter().map(|v| v * total / es).collect()
        }
        _ => vec![total / observed.len() as f64; observed.len()],
    };
    if exp.iter().any(|e| *e <= 0.0) {
        return Err("Expected values must be positive".into());
    }
    let chi: f64 = observed.iter().zip(&exp).map(|(o, e)| (o - e) * (o - e) / e).sum();
    let df = observed.len() as f64 - 1.0;
    let small = exp.iter().filter(|e| **e < 5.0).count();
    Ok(Outcome {
        statistic: ("χ²", chi),
        p: 1.0 - chi2_cdf(chi, df),
        rows: vec![("Categories".into(), observed.len() as f64), ("Total observed".into(), total), ("Degrees of freedom".into(), df)],
        ci: None,
        critical: Some(format!("χ² = {}", crate::fmt::short(inv_chi2(1.0 - alpha, df), 6))),
        note: if small > 0 {
            Some(format!("{} expected count(s) below 5: the χ² approximation may be poor", small))
        } else if expected.map(|e| e.is_empty()).unwrap_or(true) {
            Some("Expected counts taken as equal for every category".into())
        } else {
            None
        },
    })
}

/// χ² test of independence. Returns the outcome and the table of expected counts.
pub fn chi2_independence(table: &[Vec<f64>], alpha: f64) -> Result<(Outcome, Vec<Vec<f64>>), String> {
    let r = table.len();
    if r < 2 {
        return Err("Enter at least two rows".into());
    }
    let c = table[0].len();
    if c < 2 || table.iter().any(|row| row.len() != c) {
        return Err("Every row needs the same number of values, at least two".into());
    }
    let rows: Vec<f64> = table.iter().map(|row| row.iter().sum()).collect();
    let cols: Vec<f64> = (0..c).map(|j| table.iter().map(|row| row[j]).sum()).collect();
    let n: f64 = rows.iter().sum();
    if n <= 0.0 || rows.iter().chain(&cols).any(|t| *t <= 0.0) {
        return Err("Every row and column needs a positive total".into());
    }
    let expected: Vec<Vec<f64>> = (0..r).map(|i| (0..c).map(|j| rows[i] * cols[j] / n).collect()).collect();
    let mut chi = 0.0;
    for i in 0..r {
        for j in 0..c {
            let e = expected[i][j];
            chi += (table[i][j] - e).powi(2) / e;
        }
    }
    let df = ((r - 1) * (c - 1)) as f64;
    let small = expected.iter().flatten().filter(|e| **e < 5.0).count();
    // Cramér's V measures the strength of the association.
    let v = (chi / (n * ((r.min(c) - 1) as f64))).sqrt();
    Ok((
        Outcome {
            statistic: ("χ²", chi),
            p: 1.0 - chi2_cdf(chi, df),
            rows: vec![
                ("Table size".into(), (r * 100 + c) as f64),
                ("Total".into(), n),
                ("Degrees of freedom".into(), df),
                ("Cramér's V".into(), v),
            ],
            ci: None,
            critical: Some(format!("χ² = {}", crate::fmt::short(inv_chi2(1.0 - alpha, df), 6))),
            note: if small > 0 { Some(format!("{} expected count(s) below 5: the χ² approximation may be poor", small)) } else { None },
        },
        expected,
    ))
}

pub struct Anova {
    pub ss_between: f64,
    pub ss_within: f64,
    pub df_between: f64,
    pub df_within: f64,
    pub f: f64,
    pub p: f64,
    pub f_critical: f64,
    pub groups: Vec<(usize, f64, f64)>,
    pub grand_mean: f64,
}

pub fn anova_one_way(groups: &[Vec<f64>], alpha: f64) -> Result<Anova, String> {
    let groups: Vec<&Vec<f64>> = groups.iter().filter(|g| !g.is_empty()).collect();
    if groups.len() < 2 {
        return Err("ANOVA needs at least two groups".into());
    }
    let n: usize = groups.iter().map(|g| g.len()).sum();
    let k = groups.len();
    if n <= k {
        return Err("ANOVA needs more values than groups".into());
    }
    let grand = groups.iter().flat_map(|g| g.iter()).sum::<f64>() / n as f64;
    let mut ssb = 0.0;
    let mut ssw = 0.0;
    let mut info = Vec::new();
    for g in &groups {
        let m = mean(g);
        ssb += g.len() as f64 * (m - grand).powi(2);
        ssw += g.iter().map(|x| (x - m).powi(2)).sum::<f64>();
        let sd = if g.len() > 1 { var(g).sqrt() } else { f64::NAN };
        info.push((g.len(), m, sd));
    }
    let (dfb, dfw) = ((k - 1) as f64, (n - k) as f64);
    if ssw == 0.0 {
        return Err("There is no variation inside the groups, so F is undefined".into());
    }
    let f = (ssb / dfb) / (ssw / dfw);
    Ok(Anova {
        ss_between: ssb,
        ss_within: ssw,
        df_between: dfb,
        df_within: dfw,
        f,
        p: 1.0 - f_cdf(f, dfb, dfw),
        f_critical: inv_f(1.0 - alpha, dfb, dfw),
        groups: info,
        grand_mean: grand,
    })
}

fn ranks(v: &[f64]) -> Vec<f64> {
    let mut idx: Vec<usize> = (0..v.len()).collect();
    idx.sort_by(|a, b| v[*a].partial_cmp(&v[*b]).unwrap_or(std::cmp::Ordering::Equal));
    let mut r = vec![0.0; v.len()];
    let mut i = 0;
    while i < idx.len() {
        let mut j = i;
        while j + 1 < idx.len() && v[idx[j + 1]] == v[idx[i]] {
            j += 1;
        }
        let avg = (i + j) as f64 / 2.0 + 1.0;
        for k in i..=j {
            r[idx[k]] = avg;
        }
        i = j + 1;
    }
    r
}

fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let (mx, my) = (mean(x), mean(y));
    let sxy: f64 = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum();
    let sxx: f64 = x.iter().map(|a| (a - mx).powi(2)).sum();
    let syy: f64 = y.iter().map(|b| (b - my).powi(2)).sum();
    sxy / (sxx * syy).sqrt()
}

pub fn correlation(x: &[f64], y: &[f64], spearman: bool, tail: Tail, alpha: f64) -> Result<Outcome, String> {
    if x.len() != y.len() {
        return Err("Correlation needs the same number of X and Y values".into());
    }
    need(x, 3, "Correlation")?;
    let (r, name) = if spearman { (pearson(&ranks(x), &ranks(y)), "Spearman ρ") } else { (pearson(x, y), "Pearson r") };
    if !r.is_finite() {
        return Err("One of the columns has no spread".into());
    }
    let n = x.len() as f64;
    let df = n - 2.0;
    let t = if r.abs() >= 1.0 { f64::INFINITY * r.signum() } else { r * (df / (1.0 - r * r)).sqrt() };
    // Fisher z interval for the correlation.
    let ci = if !spearman && n > 3.0 && r.abs() < 1.0 {
        let z = r.atanh();
        let h = inv_norm(1.0 - alpha / 2.0) / (n - 3.0).sqrt();
        Some((format!("{}% confidence interval for ρ", (1.0 - alpha) * 100.0), (z - h).tanh(), (z + h).tanh()))
    } else {
        None
    };
    Ok(Outcome {
        statistic: ("t", t),
        p: p_from(t_cdf(t, df), tail),
        rows: vec![(name.into(), r), ("r²".into(), r * r), ("n".into(), n), ("Degrees of freedom".into(), df)],
        ci,
        critical: Some(t_critical(alpha, df, tail, "t")),
        note: Some("H₀: no correlation (ρ = 0)".into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{} vs {}", a, b);
    }

    #[test]
    fn distributions() {
        close(t_cdf(2.0, 10.0), 0.9633059826146299, 1e-10);
        close(inv_t(0.975, 10.0), 2.228138851986274, 1e-9);
        close(chi2_cdf(3.84, 1.0), 0.9499565, 1e-6);
        close(inv_chi2(0.95, 2.0), 5.991464547107979, 1e-9);
        close(f_cdf(3.0, 5.0, 10.0), 0.9344424379061559, 1e-9);
        close(inv_f(0.95, 3.0, 20.0), 3.098391212140781, 1e-8);
        close(t_pdf(0.0, 1.0), 1.0 / std::f64::consts::PI, 1e-12);
    }

    #[test]
    fn t_tests() {
        let x = [5.1, 4.9, 5.6, 5.8, 6.0, 5.4, 5.3];
        let o = t_one_sample(&x, 5.0, Tail::Two, 0.05).unwrap();
        close(o.statistic.1, 3.0301037378974924, 1e-9);
        close(o.p, 0.023094835525958694, 1e-9);
        let a = [20.0, 22.0, 19.0, 24.0, 25.0];
        let b = [28.0, 30.0, 27.0, 26.0, 29.0];
        let w = t_two_sample(&a, &b, false, Tail::Two, 0.05).unwrap();
        close(w.statistic.1, -4.47213595499958, 1e-9);
        close(w.p, 0.003247604483476812, 1e-9);
        let p = t_two_sample(&a, &b, true, Tail::Two, 0.05).unwrap();
        close(p.p, 0.0020773377112267804, 1e-10);
    }

    #[test]
    fn chi_and_anova() {
        let o = chi2_gof(&[50.0, 30.0, 20.0], None, 0.05).unwrap();
        close(o.statistic.1, 14.0, 1e-9);
        close(o.p, 0.000911882, 1e-8);
        let (ind, _) = chi2_independence(&[vec![10.0, 20.0], vec![30.0, 40.0]], 0.05).unwrap();
        close(ind.statistic.1, 0.7936507936507936, 1e-12);
        close(ind.p, 0.37299848361348686, 1e-10);
        let a = anova_one_way(&[vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0], vec![7.0, 8.0, 9.0]], 0.05).unwrap();
        close(a.f, 27.0, 1e-9);
        close(a.p, 0.001, 1e-10);
    }

    #[test]
    fn correlation_and_f() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let y = [2.0, 1.0, 4.0, 3.0, 7.0, 5.0];
        let p = correlation(&x, &y, false, Tail::Two, 0.05).unwrap();
        close(p.rows[0].1, 0.7917946548886295, 1e-12);
        close(p.p, 0.06051140336275671, 1e-9);
        let s = correlation(&x, &y, true, Tail::Two, 0.05).unwrap();
        close(s.rows[0].1, 0.8285714285714287, 1e-12);
        close(s.p, 0.04156268221574335, 1e-9);
        let f = f_test(&[20.0, 22.0, 19.0, 24.0, 25.0], &[28.0, 30.0, 27.0, 26.0, 29.0], Tail::Two, 0.05).unwrap();
        close(f.statistic.1, 2.6, 1e-12);
        close(f.p, 0.37722908093278473, 1e-9);
    }
}
