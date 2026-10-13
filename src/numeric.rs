//! Numerical methods: calculus, root finding, polynomial roots, linear systems.

use num_complex::Complex64 as C;

pub type F<'a> = &'a mut dyn FnMut(f64) -> f64;

pub fn derivative(f: F, x: f64) -> f64 {
    let h = 1e-3 * x.abs().max(1.0);
    // Richardson extrapolation on the 5-point stencil.
    let mut d = |h: f64| (f(x - 2.0 * h) - 8.0 * f(x - h) + 8.0 * f(x + h) - f(x + 2.0 * h)) / (12.0 * h);
    let d1 = d(h);
    let d2 = d(h / 2.0);
    (16.0 * d2 - d1) / 15.0
}

/// Two-sided limit of f as x → a (a may be ±∞). Samples ever closer to a from both sides,
/// extrapolates the remaining error away (Richardson) and returns the value the samples settle
/// on; None when a side blows up or the two sides disagree.
pub fn limit(f: F, a: f64) -> Option<f64> {
    let mut probe = |x: f64| -> Option<f64> {
        let v = f(x);
        v.is_finite().then_some(v)
    };
    // Round to 10 significant digits: the extrapolation leaves noise below that.
    let tidy = |v: f64| {
        if v == 0.0 || v.abs() < 1e-300 {
            return 0.0;
        }
        let mag = 10f64.powi(9 - v.abs().log10().floor() as i32);
        (v * mag).round() / mag
    };
    if a.is_infinite() {
        // Error typically shrinks like 1/x, so each step (x × 10) leaves a tenth of it.
        let sign = a.signum();
        let mut last: Option<f64> = None;
        for k in 2..=12 {
            let v = probe(sign * 10f64.powi(k))?;
            if let Some(prev) = last {
                let extrap = (10.0 * v - prev) / 9.0;
                if (v - prev).abs() <= 1e-6 * (1.0 + v.abs()) {
                    return Some(tidy(extrap));
                }
            }
            last = Some(v);
        }
        return None;
    }
    // The average of f(a−h) and f(a+h) is off by O(h²): one step (h ÷ 10) leaves a hundredth.
    let scale = 1.0 + a.abs();
    let mut last: Option<f64> = None;
    let mut sides = f64::INFINITY;
    let mut best = 0.0;
    for k in 2..=8 {
        let h = scale * 10f64.powi(-k);
        let (l, r) = (probe(a - h)?, probe(a + h)?);
        sides = (l - r).abs() / (1.0 + l.abs().max(r.abs()));
        let mid = 0.5 * (l + r);
        if let Some(prev) = last {
            best = (100.0 * mid - prev) / 99.0;
            if (mid - prev).abs() <= 1e-6 * (1.0 + mid.abs()) && sides <= 1e-5 {
                break;
            }
        } else {
            best = mid;
        }
        last = Some(mid);
    }
    // The two sides must have met: for 1/x or |x|/x they never do.
    (sides <= 1e-5).then(|| tidy(best))
}

/// The n-th derivative at x (n = 1 … 4): 5-point stencils with Richardson extrapolation.
pub fn derivative_n(f: F, x: f64, n: u32) -> f64 {
    if n <= 1 {
        return if n == 0 { f(x) } else { derivative(f, x) };
    }
    let h0 = [0.0, 0.0, 3e-2, 5e-2, 1e-1][n.min(4) as usize] * x.abs().max(1.0);
    let mut d = |h: f64| {
        let (m2, m1, p1, p2) = (f(x - 2.0 * h), f(x - h), f(x + h), f(x + 2.0 * h));
        match n {
            2 => (-p2 + 16.0 * p1 - 30.0 * f(x) + 16.0 * m1 - m2) / (12.0 * h * h),
            3 => (-m2 + 2.0 * m1 - 2.0 * p1 + p2) / (2.0 * h * h * h),
            _ => (m2 - 4.0 * m1 + 6.0 * f(x) - 4.0 * p1 + p2) / (h * h * h * h),
        }
    };
    let (d1, d2) = (d(h0), d(h0 / 2.0));
    // Error orders: h⁴ for the second derivative, h² for the third and fourth.
    if n == 2 { (16.0 * d2 - d1) / 15.0 } else { (4.0 * d2 - d1) / 3.0 }
}

/// ∫ₐᵇ f with either end allowed to be ±∞ (mapped onto a finite interval).
pub fn integrate_any(f: F, a: f64, b: f64) -> f64 {
    if a.is_finite() && b.is_finite() {
        return integrate(f, a, b);
    }
    if a.is_infinite() && b.is_infinite() {
        if a.signum() == b.signum() {
            return 0.0;
        }
        let s = b.signum();
        return s * (integrate_any(f, 0.0, f64::INFINITY) + integrate_any(f, f64::NEG_INFINITY, 0.0));
    }
    if b.is_infinite() {
        if b < 0.0 {
            return -integrate_any(f, f64::NEG_INFINITY, a);
        }
        // x = a + t/(1−t), dx = dt/(1−t)², t ∈ [0, 1)
        // Gauss–Kronrod never samples the end points, so t = 1 itself is never reached.
        let mut g = |t: f64| {
            let u = 1.0 - t;
            if u <= 0.0 { 0.0 } else { f(a + t / u) / (u * u) }
        };
        return integrate(&mut g, 0.0, 1.0);
    }
    // a = ∓∞
    if a > 0.0 {
        return -integrate_any(f, b, f64::INFINITY);
    }
    let mut g = |t: f64| {
        let u = 1.0 - t;
        if u <= 0.0 { 0.0 } else { f(b - t / u) / (u * u) }
    };
    integrate(&mut g, 0.0, 1.0)
}

pub fn integrate(f: F, a: f64, b: f64) -> f64 {
    if a == b {
        return 0.0;
    }
    // Composite adaptive Gauss–Kronrod (G7, K15) over 8 starting panels.
    let panels = 8;
    let w = (b - a) / panels as f64;
    let mut total = 0.0;
    for i in 0..panels {
        let lo = a + w * i as f64;
        total += gk_adaptive(f, lo, lo + w, 1e-11, 24);
    }
    total
}

const XGK: [f64; 8] = [
    0.991455371120812639206854697526329,
    0.949107912342758524526189684047851,
    0.864864423359769072789712788640926,
    0.741531185599394439863864773280788,
    0.586087235467691130294144838258730,
    0.405845151377397166906606412076961,
    0.207784955007898467600689403773245,
    0.000000000000000000000000000000000,
];
const WGK: [f64; 8] = [
    0.022935322010529224963732008058970,
    0.063092092629978553290700663189204,
    0.104790010322250183839876322541518,
    0.140653259715525918745189590510238,
    0.169004726639267902826583426598550,
    0.190350578064785409913256402421014,
    0.204432940075298892414161999234649,
    0.209482141084727828012999174891714,
];
const WG: [f64; 4] = [
    0.129484966168869693270611432679082,
    0.279705391489276667901467771423780,
    0.381830050505118944950369775488975,
    0.417959183673469387755102040816327,
];

fn gk15(f: F, a: f64, b: f64) -> (f64, f64) {
    let c = 0.5 * (a + b);
    let h = 0.5 * (b - a);
    let fc = f(c);
    let mut k = WGK[7] * fc;
    let mut g = WG[3] * fc;
    for i in 0..7 {
        let dx = h * XGK[i];
        let s = f(c - dx) + f(c + dx);
        k += WGK[i] * s;
        if i % 2 == 1 {
            g += WG[i / 2] * s;
        }
    }
    (k * h, ((k - g) * h).abs())
}

fn gk_adaptive(f: F, a: f64, b: f64, tol: f64, depth: u32) -> f64 {
    let (v, err) = gk15(f, a, b);
    if !v.is_finite() {
        return v;
    }
    if depth == 0 || err <= tol * v.abs().max(1.0) {
        return v;
    }
    let m = 0.5 * (a + b);
    gk_adaptive(f, a, m, tol, depth - 1) + gk_adaptive(f, m, b, tol, depth - 1)
}

fn bisect(f: F, mut a: f64, mut b: f64, mut fa: f64) -> f64 {
    for _ in 0..200 {
        let m = 0.5 * (a + b);
        if m == a || m == b {
            break;
        }
        let fm = f(m);
        if fm == 0.0 {
            return m;
        }
        if !fm.is_finite() {
            return f64::NAN;
        }
        if (fm < 0.0) == (fa < 0.0) {
            a = m;
            fa = fm;
        } else {
            b = m;
        }
    }
    0.5 * (a + b)
}

fn golden(f: F, mut a: f64, mut b: f64) -> f64 {
    // Minimises f on [a, b].
    let gr = 0.618_033_988_749_894_8;
    let mut c = b - gr * (b - a);
    let mut d = a + gr * (b - a);
    let mut fc = f(c);
    let mut fd = f(d);
    for _ in 0..120 {
        if (b - a).abs() < 1e-13 * (a.abs() + b.abs()).max(1e-3) {
            break;
        }
        if fc < fd {
            b = d;
            d = c;
            fd = fc;
            c = b - gr * (b - a);
            fc = f(c);
        } else {
            a = c;
            c = d;
            fc = fd;
            d = a + gr * (b - a);
            fd = f(d);
        }
    }
    0.5 * (a + b)
}

fn tidy(x: f64, span: f64) -> f64 {
    // Snap values that are an integer (or zero) to within numerical noise.
    let r = x.round();
    if (x - r).abs() < 1e-9 * span.max(1.0) {
        r
    } else {
        x
    }
}

/// All roots of f in [a, b], found by scanning `n` intervals.
pub fn find_roots(f: F, a: f64, b: f64, n: usize) -> Vec<f64> {
    let span = b - a;
    let mut out: Vec<f64> = Vec::new();
    if !(span > 0.0) {
        return out;
    }
    let xs: Vec<f64> = (0..=n).map(|i| a + span * i as f64 / n as f64).collect();
    let ys: Vec<f64> = xs.iter().map(|x| f(*x)).collect();
    let push = |out: &mut Vec<f64>, r: f64| {
        let r = tidy(r, span);
        if r.is_finite() && !out.iter().any(|o| (o - r).abs() < 1e-7 * span.max(1e-9)) {
            out.push(r);
        }
    };
    for i in 0..n {
        let (y0, y1) = (ys[i], ys[i + 1]);
        if !y0.is_finite() || !y1.is_finite() {
            continue;
        }
        if y0 == 0.0 {
            push(&mut out, xs[i]);
            continue;
        }
        if i == n - 1 && y1 == 0.0 {
            push(&mut out, xs[i + 1]);
            continue;
        }
        if (y0 < 0.0) != (y1 < 0.0) && y1 != 0.0 {
            let r = bisect(f, xs[i], xs[i + 1], y0);
            if r.is_finite() {
                let fr = f(r).abs();
                // Reject sign changes across a pole.
                if fr.is_finite() && fr <= 1e-6 * y0.abs().max(y1.abs()).max(1e-9) {
                    push(&mut out, r);
                }
            }
        }
    }
    // Tangent roots: local minima of |f| that touch zero.
    for i in 1..n {
        let (p, c, q) = (ys[i - 1].abs(), ys[i].abs(), ys[i + 1].abs());
        if p.is_finite() && c.is_finite() && q.is_finite() && c <= p && c <= q && (c < p || c < q) {
            let same_sign = (ys[i - 1] < 0.0) == (ys[i + 1] < 0.0);
            if same_sign {
                let r = golden(&mut |x| f(x).abs(), xs[i - 1], xs[i + 1]);
                let scale = p.max(q).max(1e-9);
                if f(r).abs() < 1e-10 * scale.max(1.0) {
                    push(&mut out, r);
                }
            }
        }
    }
    out.sort_by(|a, b| a.partial_cmp(b).unwrap());
    out
}

#[derive(Clone, Copy, Debug)]
pub struct Extremum {
    pub x: f64,
    pub y: f64,
    pub is_max: bool,
}

pub fn find_extrema(f: F, a: f64, b: f64, n: usize) -> Vec<Extremum> {
    let span = b - a;
    let mut out = Vec::new();
    if !(span > 0.0) {
        return out;
    }
    let xs: Vec<f64> = (0..=n).map(|i| a + span * i as f64 / n as f64).collect();
    let ys: Vec<f64> = xs.iter().map(|x| f(*x)).collect();
    for i in 1..n {
        let (p, c, q) = (ys[i - 1], ys[i], ys[i + 1]);
        if !p.is_finite() || !c.is_finite() || !q.is_finite() {
            continue;
        }
        let is_max = c >= p && c >= q && (c > p || c > q);
        let is_min = c <= p && c <= q && (c < p || c < q);
        if !is_max && !is_min {
            continue;
        }
        let x = if is_max {
            golden(&mut |x| -f(x), xs[i - 1], xs[i + 1])
        } else {
            golden(&mut |x| f(x), xs[i - 1], xs[i + 1])
        };
        // Golden-section is only good to ~sqrt(eps); polish by snapping.
        let x = tidy_loose(x, span);
        let y = f(x);
        if !y.is_finite() {
            continue;
        }
        // Skip cusps created by poles (value far from neighbours).
        let local = (p - c).abs().max((q - c).abs());
        if (y - c).abs() > 50.0 * local.max(1e-9) {
            continue;
        }
        out.push(Extremum { x, y, is_max });
    }
    out
}

fn tidy_loose(x: f64, span: f64) -> f64 {
    // Extremum locations are accurate to ~1e-8; snap to a short decimal when close.
    for scale in [1.0, 10.0, 100.0, 1000.0] {
        let r = (x * scale).round() / scale;
        if (x - r).abs() < 1e-7 * span.max(1.0) {
            return r;
        }
    }
    x
}

/// Solve f(x) = 0 near a starting guess.
pub fn solve_near(f: F, guess: f64) -> Option<f64> {
    let f0 = f(guess);
    if f0 == 0.0 {
        return Some(guess);
    }
    // Secant iterations first.
    let mut x0 = guess;
    let mut x1 = guess + 1e-2 * guess.abs().max(1.0);
    let mut y0 = f0;
    let mut y1 = f(x1);
    for _ in 0..80 {
        if !y0.is_finite() || !y1.is_finite() || y1 == y0 {
            break;
        }
        let x2 = x1 - y1 * (x1 - x0) / (y1 - y0);
        if !x2.is_finite() {
            break;
        }
        x0 = x1;
        y0 = y1;
        x1 = x2;
        y1 = f(x1);
        if y1 == 0.0 || (x1 - x0).abs() <= 1e-14 * x1.abs().max(1e-14) {
            if y1.abs() < 1e-8 * y0.abs().max(1.0) {
                return Some(tidy(x1, 1.0));
            }
            break;
        }
    }
    // Fall back to scanning widening windows around the guess.
    let mut w = guess.abs().max(1.0);
    for _ in 0..12 {
        let roots = find_roots(f, guess - w, guess + w, 400);
        if let Some(r) = roots
            .iter()
            .min_by(|a, b| (*a - guess).abs().partial_cmp(&(*b - guess).abs()).unwrap())
        {
            return Some(*r);
        }
        w *= 4.0;
    }
    None
}

/// Roots of a polynomial; coefficients highest degree first.
pub fn poly_roots(coef: &[f64]) -> Result<Vec<C>, String> {
    let mut c: Vec<f64> = coef.to_vec();
    while c.first().map(|v| *v == 0.0).unwrap_or(false) {
        c.remove(0);
    }
    if c.len() < 2 {
        return Err("The leading coefficients cannot all be zero".into());
    }
    let n = c.len() - 1;
    let lead = c[0];
    let a: Vec<C> = c.iter().map(|v| C::new(v / lead, 0.0)).collect();
    let eval = |z: C| a.iter().fold(C::new(0.0, 0.0), |acc, k| acc * z + k);
    let radius = 1.0 + a.iter().skip(1).fold(0.0f64, |m, k| m.max(k.norm()));
    let mut roots: Vec<C> = (0..n)
        .map(|k| C::from_polar(radius * 0.7, 2.0 * std::f64::consts::PI * k as f64 / n as f64 + 0.4))
        .collect();
    for _ in 0..2000 {
        let mut delta = 0.0f64;
        for i in 0..n {
            let mut denom = C::new(1.0, 0.0);
            for j in 0..n {
                if i != j {
                    denom *= roots[i] - roots[j];
                }
            }
            if denom.norm() == 0.0 {
                roots[i] += C::new(1e-8, 1e-8);
                delta = 1.0;
                continue;
            }
            let step = eval(roots[i]) / denom;
            roots[i] -= step;
            delta = delta.max(step.norm());
        }
        if delta < 1e-15 * radius {
            break;
        }
    }
    // Clean: snap conjugate noise and near-integers.
    let snap = |v: f64| {
        let r = (v * 1e6).round() / 1e6;
        if (v - r).abs() < 1e-10 * radius.max(1.0) {
            r
        } else {
            v
        }
    };
    let mut out: Vec<C> = roots
        .into_iter()
        .map(|z| {
            let mut z = z;
            if z.im.abs() < 1e-8 * z.norm().max(1.0) {
                z.im = 0.0;
            }
            if z.re.abs() < 1e-10 * z.norm().max(1.0) {
                z.re = 0.0;
            }
            C::new(snap(z.re), snap(z.im))
        })
        .collect();
    out.sort_by(|p, q| {
        let key = |z: &C| (z.im != 0.0, z.re, z.im);
        key(p).partial_cmp(&key(q)).unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(out)
}

#[derive(Debug, PartialEq)]
pub enum LinSol {
    Unique(Vec<f64>),
    NoSolution,
    Infinite,
}

pub fn linear_solve(a: &[Vec<f64>], b: &[f64]) -> LinSol {
    let n = b.len();
    let mut m: Vec<Vec<f64>> = a
        .iter()
        .zip(b)
        .map(|(row, bi)| {
            let mut r = row.clone();
            r.push(*bi);
            r
        })
        .collect();
    let scale = m.iter().flatten().fold(0.0f64, |acc, v| acc.max(v.abs())).max(1e-300);
    let mut rank = 0;
    let mut pivots = vec![];
    for col in 0..n {
        let mut p = rank;
        for i in rank..n {
            if m[i][col].abs() > m[p][col].abs() {
                p = i;
            }
        }
        if rank >= n || m[p][col].abs() < 1e-11 * scale {
            continue;
        }
        m.swap(p, rank);
        let piv = m[rank][col];
        for j in 0..=n {
            m[rank][j] /= piv;
        }
        for i in 0..n {
            if i != rank {
                let f = m[i][col];
                if f != 0.0 {
                    for j in 0..=n {
                        m[i][j] -= f * m[rank][j];
                    }
                }
            }
        }
        pivots.push(col);
        rank += 1;
    }
    if rank == n {
        let mut x = vec![0.0; n];
        for (i, col) in pivots.iter().enumerate() {
            x[*col] = tidy(m[i][n], 1.0);
        }
        return LinSol::Unique(x);
    }
    for row in m.iter().skip(rank) {
        if row[n].abs() > 1e-9 * scale {
            return LinSol::NoSolution;
        }
    }
    LinSol::Infinite
}
