//! Descriptive statistics, regression and probability distributions.

use crate::numeric::{linear_solve, LinSol};

#[derive(Clone, Debug, Default)]
pub struct OneVar {
    pub n: f64,
    pub mean: f64,
    pub sum: f64,
    pub sum_sq: f64,
    pub pop_sd: f64,
    pub sample_sd: f64,
    pub min: f64,
    pub q1: f64,
    pub median: f64,
    pub q3: f64,
    pub max: f64,
    pub mode: Vec<f64>,
}

pub fn median_sorted(v: &[f64]) -> f64 {
    let n = v.len();
    if n == 0 {
        return f64::NAN;
    }
    if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    }
}

/// One-variable statistics. `freq` is optional; when given it must match `xs` in length.
pub fn one_var(xs: &[f64], freq: Option<&[f64]>) -> Result<OneVar, String> {
    if xs.is_empty() {
        return Err("No data".into());
    }
    let w: Vec<f64> = match freq {
        Some(f) => {
            if f.len() != xs.len() {
                return Err("Frequency list must match the data length".into());
            }
            if f.iter().any(|v| *v < 0.0) {
                return Err("Frequencies cannot be negative".into());
            }
            f.to_vec()
        }
        None => vec![1.0; xs.len()],
    };
    let n: f64 = w.iter().sum();
    if n <= 0.0 {
        return Err("No data".into());
    }
    let sum: f64 = xs.iter().zip(&w).map(|(x, w)| x * w).sum();
    let sum_sq: f64 = xs.iter().zip(&w).map(|(x, w)| x * x * w).sum();
    let mean = sum / n;
    let ss: f64 = xs.iter().zip(&w).map(|(x, w)| (x - mean).powi(2) * w).sum();
    let pop_sd = (ss / n).sqrt();
    let sample_sd = if n > 1.0 { (ss / (n - 1.0)).sqrt() } else { f64::NAN };

    // Expand to a sorted sample for the order statistics when frequencies are whole numbers.
    let mut pairs: Vec<(f64, f64)> = xs.iter().cloned().zip(w.iter().cloned()).filter(|p| p.1 > 0.0).collect();
    pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let whole = pairs.iter().all(|p| p.1.fract() == 0.0) && n <= 2_000_000.0;
    let (q1, median, q3) = if whole {
        let mut s: Vec<f64> = Vec::with_capacity(n as usize);
        for (x, c) in &pairs {
            for _ in 0..(*c as usize) {
                s.push(*x);
            }
        }
        let len = s.len();
        let med = median_sorted(&s);
        if len < 2 {
            (med, med, med)
        } else {
            let half = len / 2;
            (median_sorted(&s[..half]), med, median_sorted(&s[len - half..]))
        }
    } else {
        let q = |p: f64| {
            let target = p * n;
            let mut acc = 0.0;
            for (x, c) in &pairs {
                acc += c;
                if acc >= target {
                    return *x;
                }
            }
            pairs.last().unwrap().0
        };
        (q(0.25), q(0.5), q(0.75))
    };

    // Mode: values with the highest total frequency (only when something repeats).
    let mut counts: Vec<(f64, f64)> = Vec::new();
    for (x, c) in &pairs {
        match counts.last_mut() {
            Some(last) if last.0 == *x => last.1 += c,
            _ => counts.push((*x, *c)),
        }
    }
    let top = counts.iter().fold(0.0f64, |m, c| m.max(c.1));
    let mode: Vec<f64> = if counts.len() > 1 && counts.iter().all(|c| c.1 == top) {
        vec![]
    } else {
        counts.iter().filter(|c| c.1 == top).map(|c| c.0).collect()
    };

    Ok(OneVar {
        n,
        mean,
        sum,
        sum_sq,
        pop_sd,
        sample_sd,
        min: pairs.first().unwrap().0,
        q1,
        median,
        q3,
        max: pairs.last().unwrap().0,
        mode,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Model {
    Linear,
    Quadratic,
    Cubic,
    Exponential,
    Logarithmic,
    Power,
}

impl Model {
    pub const ALL: [Model; 6] =
        [Model::Linear, Model::Quadratic, Model::Cubic, Model::Exponential, Model::Logarithmic, Model::Power];

    pub fn name(self) -> &'static str {
        match self {
            Model::Linear => "Linear",
            Model::Quadratic => "Quadratic",
            Model::Cubic => "Cubic",
            Model::Exponential => "Exponential",
            Model::Logarithmic => "Logarithmic",
            Model::Power => "Power",
        }
    }

    pub fn formula(self) -> &'static str {
        match self {
            Model::Linear => "y = a·x + b",
            Model::Quadratic => "y = a·x² + b·x + c",
            Model::Cubic => "y = a·x³ + b·x² + c·x + d",
            Model::Exponential => "y = a·e^(b·x)",
            Model::Logarithmic => "y = a + b·ln(x)",
            Model::Power => "y = a·x^b",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Regression {
    pub model: Model,
    pub coef: Vec<f64>,
    pub r: Option<f64>,
    pub r2: f64,
}

impl Regression {
    pub fn predict(&self, x: f64) -> f64 {
        let c = &self.coef;
        match self.model {
            Model::Linear => c[0] * x + c[1],
            Model::Quadratic => (c[0] * x + c[1]) * x + c[2],
            Model::Cubic => ((c[0] * x + c[1]) * x + c[2]) * x + c[3],
            Model::Exponential => c[0] * (c[1] * x).exp(),
            Model::Logarithmic => c[0] + c[1] * x.ln(),
            Model::Power => c[0] * x.powf(c[1]),
        }
    }
}

fn polyfit(xs: &[f64], ys: &[f64], deg: usize) -> Option<Vec<f64>> {
    let m = deg + 1;
    if xs.len() < m {
        return None;
    }
    // Normal equations on centred/scaled x for conditioning.
    let mean = xs.iter().sum::<f64>() / xs.len() as f64;
    let sd = (xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / xs.len() as f64).sqrt();
    if sd == 0.0 {
        return None;
    }
    let t: Vec<f64> = xs.iter().map(|x| (x - mean) / sd).collect();
    let mut a = vec![vec![0.0; m]; m];
    let mut b = vec![0.0; m];
    for (ti, yi) in t.iter().zip(ys) {
        let mut pw = vec![1.0; 2 * m];
        for k in 1..2 * m {
            pw[k] = pw[k - 1] * ti;
        }
        for i in 0..m {
            for j in 0..m {
                a[i][j] += pw[i + j];
            }
            b[i] += pw[i] * yi;
        }
    }
    let c = match linear_solve(&a, &b) {
        LinSol::Unique(c) => c,
        _ => return None,
    };
    // c is ascending in t; expand back to descending powers of x.
    // p(x) = Σ c_k ((x-mean)/sd)^k
    let mut out = vec![0.0; m]; // ascending in x
    for (k, ck) in c.iter().enumerate() {
        // ((x - mean)/sd)^k = Σ_j C(k,j) x^j (-mean)^(k-j) / sd^k
        let mut binom = 1.0;
        for j in 0..=k {
            if j > 0 {
                binom = binom * (k - j + 1) as f64 / j as f64;
            }
            out[j] += ck * binom * (-mean).powi((k - j) as i32) / sd.powi(k as i32);
        }
    }
    out.reverse();
    Some(out)
}

pub fn regress(model: Model, xs: &[f64], ys: &[f64]) -> Result<Regression, String> {
    if xs.len() != ys.len() {
        return Err("X and Y need the same number of values".into());
    }
    let need = match model {
        Model::Quadratic => 3,
        Model::Cubic => 4,
        _ => 2,
    };
    if xs.len() < need {
        return Err(format!("Need at least {} data points", need));
    }
    let fail = || "Cannot fit this model to the data".to_string();
    let (tx, ty): (Vec<f64>, Vec<f64>) = match model {
        Model::Exponential => {
            if ys.iter().any(|y| *y <= 0.0) {
                return Err("Exponential fit needs y > 0".into());
            }
            (xs.to_vec(), ys.iter().map(|y| y.ln()).collect())
        }
        Model::Logarithmic => {
            if xs.iter().any(|x| *x <= 0.0) {
                return Err("Logarithmic fit needs x > 0".into());
            }
            (xs.iter().map(|x| x.ln()).collect(), ys.to_vec())
        }
        Model::Power => {
            if xs.iter().any(|x| *x <= 0.0) || ys.iter().any(|y| *y <= 0.0) {
                return Err("Power fit needs x > 0 and y > 0".into());
            }
            (xs.iter().map(|x| x.ln()).collect(), ys.iter().map(|y| y.ln()).collect())
        }
        _ => (xs.to_vec(), ys.to_vec()),
    };
    let deg = match model {
        Model::Quadratic => 2,
        Model::Cubic => 3,
        _ => 1,
    };
    let p = polyfit(&tx, &ty, deg).ok_or_else(fail)?;
    let coef = match model {
        Model::Exponential | Model::Power => vec![p[1].exp(), p[0]],
        Model::Logarithmic => vec![p[1], p[0]],
        _ => p.clone(),
    };
    // Goodness of fit on the (possibly transformed) data, as Casio reports it.
    let n = tx.len() as f64;
    let my = ty.iter().sum::<f64>() / n;
    let mx = tx.iter().sum::<f64>() / n;
    let ss_tot: f64 = ty.iter().map(|y| (y - my).powi(2)).sum();
    let ss_res: f64 = tx
        .iter()
        .zip(&ty)
        .map(|(x, y)| {
            let fit = p.iter().fold(0.0, |acc, c| acc * x + c);
            (y - fit).powi(2)
        })
        .sum();
    let r2 = if ss_tot == 0.0 { 1.0 } else { 1.0 - ss_res / ss_tot };
    let r = if deg == 1 {
        let sxy: f64 = tx.iter().zip(&ty).map(|(x, y)| (x - mx) * (y - my)).sum();
        let sxx: f64 = tx.iter().map(|x| (x - mx).powi(2)).sum();
        if sxx == 0.0 || ss_tot == 0.0 {
            None
        } else {
            Some(sxy / (sxx * ss_tot).sqrt())
        }
    } else {
        None
    };
    Ok(Regression { model, coef, r, r2 })
}

// ---------- Distributions ----------

pub fn norm_pdf(x: f64, mu: f64, sigma: f64) -> f64 {
    let z = (x - mu) / sigma;
    (-0.5 * z * z).exp() / (sigma * (2.0 * std::f64::consts::PI).sqrt())
}

pub fn norm_cdf(x: f64, mu: f64, sigma: f64) -> f64 {
    0.5 * libm::erfc(-(x - mu) / (sigma * std::f64::consts::SQRT_2))
}

/// Inverse of the standard normal CDF (Acklam's approximation, polished with one Halley step).
pub fn inv_norm(p: f64) -> f64 {
    if !(p > 0.0 && p < 1.0) {
        return if p == 0.0 {
            f64::NEG_INFINITY
        } else if p == 1.0 {
            f64::INFINITY
        } else {
            f64::NAN
        };
    }
    const A: [f64; 6] = [
        -3.969683028665376e+01,
        2.209460984245205e+02,
        -2.759285104469687e+02,
        1.383577518672690e+02,
        -3.066479806614716e+01,
        2.506628277459239e+00,
    ];
    const B: [f64; 5] = [
        -5.447609879822406e+01,
        1.615858368580409e+02,
        -1.556989798598866e+02,
        6.680131188771972e+01,
        -1.328068155288572e+01,
    ];
    const CC: [f64; 6] = [
        -7.784894002430293e-03,
        -3.223964580411365e-01,
        -2.400758277161838e+00,
        -2.549732539343734e+00,
        4.374664141464968e+00,
        2.938163982698783e+00,
    ];
    const D: [f64; 4] = [7.784695709041462e-03, 3.224671290700398e-01, 2.445134137142996e+00, 3.754408661907416e+00];
    let pl = 0.02425;
    let x = if p < pl {
        let q = (-2.0 * p.ln()).sqrt();
        (((((CC[0] * q + CC[1]) * q + CC[2]) * q + CC[3]) * q + CC[4]) * q + CC[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p <= 1.0 - pl {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((CC[0] * q + CC[1]) * q + CC[2]) * q + CC[3]) * q + CC[4]) * q + CC[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    };
    let e = norm_cdf(x, 0.0, 1.0) - p;
    let u = e * (2.0 * std::f64::consts::PI).sqrt() * (x * x / 2.0).exp();
    x - u / (1.0 + x * u / 2.0)
}

fn ln_choose(n: f64, k: f64) -> f64 {
    libm::lgamma(n + 1.0) - libm::lgamma(k + 1.0) - libm::lgamma(n - k + 1.0)
}

pub fn binom_pdf(k: f64, n: f64, p: f64) -> f64 {
    if k < 0.0 || k > n || k.fract() != 0.0 {
        return 0.0;
    }
    if p == 0.0 {
        return if k == 0.0 { 1.0 } else { 0.0 };
    }
    if p == 1.0 {
        return if k == n { 1.0 } else { 0.0 };
    }
    (ln_choose(n, k) + k * p.ln() + (n - k) * (1.0 - p).ln()).exp()
}

pub fn binom_cdf(k: f64, n: f64, p: f64) -> f64 {
    let top = k.floor().min(n);
    let mut s = 0.0;
    let mut i = 0.0;
    while i <= top {
        s += binom_pdf(i, n, p);
        i += 1.0;
    }
    s.min(1.0)
}

pub fn poisson_pdf(k: f64, lambda: f64) -> f64 {
    if k < 0.0 || k.fract() != 0.0 {
        return 0.0;
    }
    (k * lambda.ln() - lambda - libm::lgamma(k + 1.0)).exp()
}

pub fn poisson_cdf(k: f64, lambda: f64) -> f64 {
    let top = k.floor();
    let mut s = 0.0;
    let mut i = 0.0;
    while i <= top {
        s += poisson_pdf(i, lambda);
        i += 1.0;
    }
    s.min(1.0)
}
