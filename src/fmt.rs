//! Turning values into text: display formats, exact forms, base conversion.

use crate::engine::Value;
use num_complex::Complex64 as C;

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// Digit grouping: 0 none, 1 international (1,234,567), 2 Indian (12,34,567), 3 space (1 234 567).
static GROUPING: AtomicU8 = AtomicU8::new(0);
static DECIMAL_COMMA: AtomicBool = AtomicBool::new(false);

/// Choose how numbers are written on screen. Typed input and copied text are not affected.
pub fn set_notation(grouping: u8, decimal_comma: bool) {
    GROUPING.store(grouping, Ordering::Relaxed);
    DECIMAL_COMMA.store(decimal_comma, Ordering::Relaxed);
}

/// Apply the grouping and decimal mark to the numbers in already formatted text.
pub fn localize(s: &str) -> String {
    localize_with(s, GROUPING.load(Ordering::Relaxed), DECIMAL_COMMA.load(Ordering::Relaxed))
}

fn localize_with(s: &str, grouping: u8, comma: bool) -> String {
    if grouping == 0 && !comma {
        return s.to_string();
    }
    let sep = match (grouping, comma) {
        (3, _) => ' ',
        (_, true) => '.',
        _ => ',',
    };
    let ch: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len() + 8);
    let mut i = 0;
    while i < ch.len() {
        if ch[i].is_ascii_digit() {
            let start = i;
            while i < ch.len() && ch[i].is_ascii_digit() {
                i += 1;
            }
            let run = &ch[start..i];
            let after_point = start > 0 && ch[start - 1] == '.';
            // Only whole-number parts are grouped, and not the exponent digits of 1E5.
            let after_e = start > 0 && ch[start - 1] == 'E';
            if grouping == 0 || after_point || after_e || run.len() < 4 {
                out.extend(run);
                continue;
            }
            let n = run.len();
            for (k, c) in run.iter().enumerate() {
                let from_right = n - k; // digits remaining including this one
                if k > 0 {
                    let boundary = match grouping {
                        2 => from_right == 3 || (from_right > 3 && (from_right - 3) % 2 == 0),
                        _ => from_right % 3 == 0,
                    };
                    if boundary {
                        out.push(sep);
                    }
                }
                out.push(*c);
            }
        } else if ch[i] == '.' && comma && i + 1 < ch.len() && ch[i + 1].is_ascii_digit() {
            out.push(',');
            i += 1;
        } else {
            out.push(ch[i]);
            i += 1;
        }
    }
    out
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Fmt {
    Norm,
    Fix(u8),
    Sci(u8),
}

fn superscript(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '0' => '⁰',
            '1' => '¹',
            '2' => '²',
            '3' => '³',
            '4' => '⁴',
            '5' => '⁵',
            '6' => '⁶',
            '7' => '⁷',
            '8' => '⁸',
            '9' => '⁹',
            '-' => '⁻',
            other => other,
        })
        .collect()
}

const GREEK_NAMES: [(&str, char, char); 24] = [
    // The constant π, written out.
    ("pi", 'π', 'π'),
    ("alpha", 'α', 'Α'),
    ("beta", 'β', 'Β'),
    ("gamma", 'γ', 'Γ'),
    ("delta", 'δ', 'Δ'),
    ("epsilon", 'ε', 'Ε'),
    ("zeta", 'ζ', 'Ζ'),
    ("eta", 'η', 'Η'),
    ("theta", 'θ', 'Θ'),
    ("iota", 'ι', 'Ι'),
    ("kappa", 'κ', 'Κ'),
    ("lambda", 'λ', 'Λ'),
    ("mu", 'μ', 'Μ'),
    ("nu", 'ν', 'Ν'),
    ("xi", 'ξ', 'Ξ'),
    ("omicron", 'ο', 'Ο'),
    ("rho", 'ρ', 'Ρ'),
    ("sigma", 'σ', 'Σ'),
    ("tau", 'τ', 'Τ'),
    ("upsilon", 'υ', 'Υ'),
    ("phi", 'φ', 'Φ'),
    ("chi", 'χ', 'Χ'),
    ("psi", 'ψ', 'Ψ'),
    ("omega", 'ω', 'Ω'),
];

/// Turn spelled-out Greek letters into the letters: "2alpha+Delta" becomes "2α+Δ".
/// A name only counts when it stands on its own (no letter directly before or after).
/// "gamma(" is left alone because it is the gamma function.
fn greek(s: &str, finalize: bool) -> String {
    let ch: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    'outer: while i < ch.len() {
        let c = ch[i];
        let boundary = i == 0 || !ch[i - 1].is_alphabetic();
        if boundary && c.is_ascii_alphabetic() {
            // Longest name first so "theta" wins over "eta"-like overlaps.
            let mut best: Option<(usize, char)> = None;
            for (name, low, up) in GREEK_NAMES.iter() {
                let len = name.len();
                if i + len > ch.len() || best.map(|b| b.0 >= len).unwrap_or(false) {
                    continue;
                }
                let lower_match = name.chars().zip(&ch[i..i + len]).all(|(a, b)| a == *b);
                let upper_match = !lower_match
                    && name.chars().enumerate().zip(&ch[i..i + len]).all(|((k, a), b)| {
                        if k == 0 {
                            a.to_ascii_uppercase() == *b
                        } else {
                            a == *b
                        }
                    });
                if lower_match || upper_match {
                    best = Some((len, if lower_match { *low } else { *up }));
                }
            }
            if let Some((len, letter)) = best {
                let next = ch.get(i + len).copied();
                let is_gamma = letter == 'γ';
                let ok = match next {
                    Some(n) if n.is_alphabetic() => false,
                    Some('(') if is_gamma => false,
                    Some(_) => true,
                    // "gamma" at the very end may still become "gamma(".
                    None => !is_gamma || finalize,
                };
                if ok {
                    out.push(letter);
                    i += len;
                    continue 'outer;
                }
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Tidy typed maths: spelled-out Greek letters become letters and typed powers become
/// raised digits ("x^12+1" becomes "x¹²+1").
/// A power still being typed at the end of the text is left alone unless `finalize` is set.
pub fn prettify(s: &str, finalize: bool) -> String {
    let greeked;
    let s = if s.chars().any(|c| c.is_ascii_alphabetic()) {
        greeked = greek(s, finalize);
        greeked.as_str()
    } else {
        s
    };
    if !s.contains('^') {
        return s.to_string();
    }
    let ch: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < ch.len() {
        if ch[i] == '^' {
            let mut j = i + 1;
            let neg = j < ch.len() && (ch[j] == '-' || ch[j] == '−');
            if neg {
                j += 1;
            }
            let start = j;
            while j < ch.len() && ch[j].is_ascii_digit() {
                j += 1;
            }
            let done = if j < ch.len() { ch[j] != '.' && ch[j] != '^' && ch[j] != 'E' } else { finalize };
            if j > start && done {
                if neg {
                    out.push('⁻');
                }
                let digits: String = ch[start..j].iter().collect();
                out.push_str(&superscript(&digits));
                i = j;
                continue;
            }
        }
        out.push(ch[i]);
        i += 1;
    }
    out
}

fn trim_zeros(s: &mut String) {
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
}

fn sci(x: f64, digits: usize, trim: bool, pretty: bool) -> String {
    let raw = format!("{:.*e}", digits, x);
    let (mant, exp) = raw.split_once('e').unwrap_or((&raw, "0"));
    let mut mant = mant.to_string();
    if trim {
        trim_zeros(&mut mant);
    }
    if pretty {
        format!("{}×10{}", mant.replace('-', "−"), superscript(exp))
    } else {
        format!("{}E{}", mant, exp)
    }
}

fn norm(x: f64, sig: i32, pretty: bool) -> String {
    if x == 0.0 {
        return "0".into();
    }
    let e = x.abs().log10().floor() as i32;
    // Very small numbers switch to ×10ⁿ early: 5.67×10⁻⁸ reads better than 0.0000000567.
    let low = (-(sig - 1)).max(-5);
    if e >= sig || e < low {
        return sci(x, (sig - 1) as usize, true, pretty);
    }
    let dec = (sig - 1 - e).max(0) as usize;
    let mut s = format!("{:.*}", dec, x);
    trim_zeros(&mut s);
    if s == "-0" {
        s = "0".into();
    }
    if pretty {
        s.replace('-', "−")
    } else {
        s
    }
}

pub fn real(x: f64, f: Fmt) -> String {
    localize(&real_raw(x, f))
}

fn real_raw(x: f64, f: Fmt) -> String {
    if x.is_nan() {
        return "undefined".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "∞".into() } else { "−∞".into() };
    }
    match f {
        Fmt::Norm => norm(x, 10, true),
        Fmt::Fix(n) => {
            let s = format!("{:.*}", n as usize, x);
            let zero = s.trim_start_matches('-').chars().all(|c| c == '0' || c == '.');
            if zero {
                s.trim_start_matches('-').to_string()
            } else {
                s.replace('-', "−")
            }
        }
        Fmt::Sci(n) => sci(x, n as usize, false, true),
    }
}

/// Compact form for tables and axis read-outs.
pub fn short(x: f64, sig: i32) -> String {
    localize(&short_raw(x, sig))
}

fn short_raw(x: f64, sig: i32) -> String {
    if x.is_nan() {
        return "–".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "∞".into() } else { "−∞".into() };
    }
    norm(x, sig, true)
}

pub fn complex(z: C, f: Fmt) -> String {
    if z.im == 0.0 {
        return real(z.re, f);
    }
    let im_abs = real(z.im.abs(), f);
    let im_part = if im_abs == "1" { "i".to_string() } else { format!("{}i", im_abs) };
    if z.re == 0.0 {
        return if z.im < 0.0 { format!("−{}", im_part) } else { im_part };
    }
    format!("{} {} {}", real(z.re, f), if z.im < 0.0 { "−" } else { "+" }, im_part)
}

pub fn value(v: &Value, f: Fmt) -> String {
    match v {
        Value::Num(z) => complex(*z, f),
        Value::List(l) => {
            let items: Vec<String> = l.iter().map(|x| real(*x, f)).collect();
            format!("{{{}}}", items.join(", "))
        }
        Value::Mat(m) => {
            let rows: Vec<String> = (0..m.r)
                .map(|i| {
                    let items: Vec<String> = (0..m.c).map(|j| short(m.get(i, j), 8)).collect();
                    format!("[{}]", items.join(", "))
                })
                .collect();
            format!("[{}]", rows.join(", "))
        }
    }
}

/// Text that the parser accepts again (used when a result is re-inserted).
pub fn plain(v: &Value) -> String {
    let num = |x: f64| norm(x, 13, false);
    match v {
        Value::Num(z) => {
            if z.im == 0.0 {
                num(z.re)
            } else if z.re == 0.0 {
                format!("({}i)", num(z.im))
            } else {
                format!("({}{}{}i)", num(z.re), if z.im < 0.0 { "-" } else { "+" }, num(z.im.abs()))
            }
        }
        Value::List(l) => format!("{{{}}}", l.iter().map(|x| num(*x)).collect::<Vec<_>>().join(",")),
        Value::Mat(m) => {
            let rows: Vec<String> = (0..m.r)
                .map(|i| format!("[{}]", (0..m.c).map(|j| num(m.get(i, j))).collect::<Vec<_>>().join(",")))
                .collect();
            format!("[{}]", rows.join(","))
        }
    }
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// Best rational approximation p/q with q ≤ max_den, if it matches x to within rounding.
pub fn rational(x: f64, max_den: u64) -> Option<(i64, u64)> {
    if !x.is_finite() || x.abs() > 1e9 {
        return None;
    }
    let sign = if x < 0.0 { -1 } else { 1 };
    let ax = x.abs();
    let (mut h0, mut h1) = (0u64, 1u64);
    let (mut k0, mut k1) = (1u64, 0u64);
    let mut y = ax;
    for _ in 0..40 {
        let a = y.floor();
        if a > 1e12 {
            break;
        }
        let h2 = (a as u64).checked_mul(h1)?.checked_add(h0)?;
        let k2 = (a as u64).checked_mul(k1)?.checked_add(k0)?;
        if k2 > max_den {
            break;
        }
        h0 = h1;
        h1 = h2;
        k0 = k1;
        k1 = k2;
        if (ax - h1 as f64 / k1 as f64).abs() <= 2e-13 * ax.max(1.0) {
            return Some((sign * h1 as i64, k1));
        }
        let fr = y - a;
        if fr < 1e-14 {
            break;
        }
        y = 1.0 / fr;
    }
    None
}

fn frac_text(p: i64, q: u64, unit: &str) -> String {
    let sign = if p < 0 { "−" } else { "" };
    let p = p.unsigned_abs();
    let num = if unit.is_empty() {
        p.to_string()
    } else if p == 1 {
        unit.to_string()
    } else {
        format!("{}{}", p, unit)
    };
    if q == 1 {
        format!("{}{}", sign, num)
    } else {
        format!("{}{}/{}", sign, num, q)
    }
}

/// An exact-looking form of a real number: a fraction, a multiple of π, or a surd.
pub fn exact(x: f64) -> Option<String> {
    if !x.is_finite() || x == 0.0 || x.fract() == 0.0 || x.abs() > 1e7 || x.abs() < 1e-7 {
        return None;
    }
    if let Some((p, q)) = rational(x, 9999) {
        // A whole number carrying only rounding noise has no separate exact form.
        if q == 1 {
            return None;
        }
        // A fraction with a huge numerator (1234567891/500) is no easier to read than the decimal.
        if p.unsigned_abs() >= 100_000 {
            return None;
        }
        return Some(frac_text(p, q, ""));
    }
    if let Some((p, q)) = rational(x / std::f64::consts::PI, 360) {
        if p.abs() <= 720 {
            return Some(frac_text(p, q, "π"));
        }
    }
    // x = ±√(p/q) = ±√(p·q)/q, then pull square factors out of the radicand.
    if let Some((p, q)) = rational(x * x, 400) {
        if p > 0 && p <= 100_000 {
            let n = (p as u64) * q;
            let mut outside = 1u64;
            let mut inside = n;
            let mut f = 2u64;
            while f * f <= inside {
                while inside % (f * f) == 0 {
                    inside /= f * f;
                    outside *= f;
                }
                f += 1;
            }
            if inside > 1 && inside <= 9999 {
                let g = gcd(outside, q);
                let sign = if x < 0.0 { -1 } else { 1 };
                return Some(frac_text(sign * (outside / g) as i64, q / g, &format!("√{}", inside)));
            }
        }
    }
    None
}

pub fn to_base(v: i64, radix: u32) -> String {
    let digits = b"0123456789ABCDEF";
    if radix == 10 {
        return v.to_string();
    }
    let mut u = v as u64;
    if u == 0 {
        return "0".into();
    }
    let mut out = Vec::new();
    while u > 0 {
        out.push(digits[(u % radix as u64) as usize]);
        u /= radix as u64;
    }
    out.reverse();
    String::from_utf8(out).unwrap()
}

pub fn from_base(s: &str, radix: u32) -> Option<i64> {
    let t: String = s.chars().filter(|c| !c.is_whitespace() && *c != '_').collect();
    if t.is_empty() {
        return None;
    }
    if radix == 10 {
        return t.replace('−', "-").parse::<i64>().ok();
    }
    u64::from_str_radix(&t, radix).ok().map(|u| u as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notation() {
        assert_eq!(localize_with("1234567.891", 1, false), "1,234,567.891");
        assert_eq!(localize_with("1234567.891", 2, false), "12,34,567.891");
        assert_eq!(localize_with("1234567", 3, false), "1 234 567");
        assert_eq!(localize_with("−1234.5", 1, true), "−1.234,5");
        assert_eq!(localize_with("1.5E12", 1, false), "1.5E12");
        assert_eq!(localize_with("999", 2, false), "999");
        assert_eq!(localize_with("123456", 2, false), "1,23,456");
        assert_eq!(localize_with("0.5", 0, true), "0,5");
    }

    #[test]
    fn numbers() {
        assert_eq!(real(0.1 + 0.2, Fmt::Norm), "0.3");
        assert_eq!(real(1.0 / 3.0, Fmt::Norm), "0.3333333333");
        assert_eq!(real(-2.5, Fmt::Norm), "−2.5");
        assert_eq!(real(1e12, Fmt::Norm), "1×10¹²");
        assert_eq!(real(1.5e-12, Fmt::Norm), "1.5×10⁻¹²");
        assert_eq!(real(123456789.0, Fmt::Norm), "123456789");
        assert_eq!(real(2.0 / 3.0, Fmt::Fix(2)), "0.67");
        assert_eq!(real(12345.678, Fmt::Sci(3)), "1.235×10⁴");
        assert_eq!(complex(C::new(3.0, -4.0), Fmt::Norm), "3 − 4i");
        assert_eq!(complex(C::new(0.0, 1.0), Fmt::Norm), "i");
        assert_eq!(plain(&Value::real(1e15)), "1E15");
    }

    #[test]
    fn exact_forms() {
        assert_eq!(exact(0.75).unwrap(), "3/4");
        assert_eq!(exact(-1.0 / 3.0).unwrap(), "−1/3");
        assert_eq!(exact(std::f64::consts::PI / 6.0).unwrap(), "π/6");
        assert_eq!(exact(2.0 * std::f64::consts::PI).unwrap(), "2π");
        assert_eq!(exact(2f64.sqrt() / 2.0).unwrap(), "√2/2");
        assert_eq!(exact(12f64.sqrt()).unwrap(), "2√3");
        assert_eq!(exact(3f64.sqrt() / 2.0).unwrap(), "√3/2");
        assert!(exact(std::f64::consts::E).is_none());
        assert!(exact(5.0).is_none());
    }

    #[test]
    fn pretty_powers() {
        assert_eq!(prettify("x^2+1", false), "x²+1");
        assert_eq!(prettify("x^12", false), "x^12");
        assert_eq!(prettify("x^12", true), "x¹²");
        assert_eq!(prettify("x^-2)", false), "x⁻²)");
        assert_eq!(prettify("x^2.5+1", true), "x^2.5+1");
        assert_eq!(prettify("2^(n+1)", true), "2^(n+1)");
        assert_eq!(prettify("2^3^2 ", true), "2^3² ");
    }

    #[test]
    fn pi_name() {
        assert_eq!(prettify("2pi+sin(pi/6)", true), "2π+sin(π/6)");
        assert_eq!(prettify("spin", true), "spin");
    }

    #[test]
    fn greek_names() {
        assert_eq!(prettify("2alpha+beta", false), "2α+β");
        assert_eq!(prettify("4cos(3theta)", false), "4cos(3θ)");
        assert_eq!(prettify("Delta^2-omega", true), "Δ²-ω");
        assert_eq!(prettify("gamma(5)", true), "gamma(5)");
        assert_eq!(prettify("gamma", false), "gamma");
        assert_eq!(prettify("2gamma+1", false), "2γ+1");
        assert_eq!(prettify("epsilon", false), "ε");
        assert_eq!(prettify("epsi", false), "epsi");
        assert_eq!(prettify("beta", false), "β");
        assert_eq!(prettify("mean(List1)", true), "mean(List1)");
        assert_eq!(prettify("sin(x)+tan(x)", true), "sin(x)+tan(x)");
    }

    #[test]
    fn bases() {
        assert_eq!(to_base(255, 16), "FF");
        assert_eq!(to_base(5, 2), "101");
        assert_eq!(to_base(-1, 16), "FFFFFFFFFFFFFFFF");
        assert_eq!(from_base("ff", 16), Some(255));
        assert_eq!(from_base("1010", 2), Some(10));
    }
}
