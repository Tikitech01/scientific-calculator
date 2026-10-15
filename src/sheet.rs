//! The sheet: a plain text of calculations, one per line, evaluated top to bottom.
//!
//! `name = expression` stores the value under that name (letters and digits, so fc, ωn and u1
//! all work). Anything after # is a comment. Sheets are saved as plain text files.

use crate::engine::{self, Env, Value};
use crate::fmt;

/// The built-in sheets under File ▸ New from template.
pub const TEMPLATES: [(&str, &str); 5] = [
    ("RC Filter", include_str!("sheets/rc-filter.calc")),
    ("Beam deflection", include_str!("sheets/beam-deflection.calc")),
    ("Vibration", include_str!("sheets/vibration.calc")),
    ("RLC impedance", include_str!("sheets/rlc-impedance.calc")),
    ("Measurement uncertainty", include_str!("sheets/uncertainty.calc")),
];

/// File name extension of a saved sheet.
pub const EXT: &str = "calc";

/// What one line of the sheet came to.
#[derive(Clone, Debug, PartialEq)]
pub enum Line {
    /// Blank or a comment.
    Nothing,
    Value(Value),
    Error(String),
}

/// Split a line into its calculation and its comment.
pub fn code(line: &str) -> &str {
    line.split('#').next().unwrap_or("").trim()
}

/// Split "name = expression" into the two parts, when the line is written that way.
pub fn assignment(code: &str) -> Option<(&str, &str)> {
    let (lhs, rhs) = code.split_once('=')?;
    let name = lhs.trim();
    if name.is_empty() || name.chars().any(char::is_whitespace) || rhs.contains('=') {
        return None;
    }
    Some((name, rhs.trim()))
}

/// Evaluate every line in order. Stored names stay in `env` for the rest of the app.
pub fn run(text: &str, env: &mut Env) -> Vec<Line> {
    text.lines()
        .map(|line| {
            let code = code(line);
            if code.is_empty() {
                return Line::Nothing;
            }
            match assignment(code) {
                Some((name, rhs)) => {
                    if !engine::user_name_ok(name) {
                        return Line::Error(format!("“{}” cannot be a name", name));
                    }
                    if rhs.is_empty() {
                        return Line::Nothing;
                    }
                    match engine::evaluate(rhs, env) {
                        Ok(v) => {
                            env.vars.insert(name.to_string(), v.clone());
                            Line::Value(v)
                        }
                        Err(e) => Line::Error(e),
                    }
                }
                None => match engine::evaluate(code, env) {
                    Ok(v) => Line::Value(v),
                    Err(e) => Line::Error(e),
                },
            }
        })
        .collect()
}

/// The sheet as rows for a CSV file: line, result.
pub fn csv_rows(text: &str, lines: &[Line]) -> Vec<Vec<String>> {
    text.lines()
        .zip(lines)
        .map(|(src, l)| {
            let result = match l {
                Line::Value(v) => fmt::plain(v),
                Line::Error(e) => e.clone(),
                Line::Nothing => String::new(),
            };
            vec![src.to_string(), result]
        })
        .collect()
}

/// Rows as a CSV file: commas between the cells, quotes where a cell needs them.
pub fn to_csv(head: &[&str], rows: &[Vec<String>]) -> String {
    let cell = |c: &str| {
        let c = c.replace('−', "-").replace('\u{2009}', "");
        if c.contains([',', '"', '\n']) {
            format!("\"{}\"", c.replace('"', "\"\""))
        } else {
            c
        }
    };
    let mut out = head.iter().map(|h| cell(h)).collect::<Vec<_>>().join(",");
    for r in rows {
        out.push('\n');
        out.push_str(&r.iter().map(|c| cell(c)).collect::<Vec<_>>().join(","));
    }
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(l: &Line) -> f64 {
        match l {
            Line::Value(v) => v.as_real().unwrap_or(f64::NAN),
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn names_and_comments() {
        let mut env = Env::default();
        let out = run("# RC\nR = 4.7kΩ  # ohms\nC = 100nF\nfc = 1/(2π·R·C)\n\nfc·2\nbad = = 1", &mut env);
        assert_eq!(out[0], Line::Nothing);
        assert!((value(&out[1]) - 4700.0).abs() < 1e-9);
        assert!((value(&out[3]) - 338.627).abs() < 1e-3);
        assert_eq!(out[4], Line::Nothing);
        assert!((value(&out[5]) - 677.255).abs() < 1e-3);
        assert!(matches!(out[6], Line::Error(_)));
        assert!(matches!(run("sin = 2", &mut env)[0], Line::Error(_)));
    }

    #[test]
    fn templates_run_clean() {
        for (name, text) in TEMPLATES {
            let mut env = Env::default();
            for (i, l) in run(text, &mut env).iter().enumerate() {
                assert!(!matches!(l, Line::Error(_)), "{} line {}: {:?}", name, i + 1, l);
            }
        }
    }

    #[test]
    fn template_numbers() {
        let mut env = Env::default();
        let get = |env: &Env, n: &str| env.vars[n].as_real().unwrap();
        run(TEMPLATES[0].1, &mut env);
        assert!((get(&env, "fc") - 338.627).abs() < 1e-2);
        assert!((get(&env, "G") + 9.88).abs() < 0.02);
        assert!((get(&env, "φ") + 71.29).abs() < 0.02);
        run(TEMPLATES[1].1, &mut env);
        // Cantilever, point load: δ = PL³/(3EI) with I = 0.05·0.1³/12
        let i = 0.05 * 0.1f64.powi(3) / 12.0;
        assert!((get(&env, "I") - i).abs() < 1e-12);
        assert!((get(&env, "δ1") - 10e3 * 8.0 / (3.0 * 200e9 * i)).abs() < 1e-9);
        run(TEMPLATES[2].1, &mut env);
        assert!((get(&env, "ωn") - 50.0).abs() < 1e-9);
        assert!((get(&env, "ζ") - 0.1).abs() < 1e-9);
        run(TEMPLATES[3].1, &mut env);
        assert!((get(&env, "f0") - 1591.549).abs() < 1e-2);
        run(TEMPLATES[4].1, &mut env);
        let u4 = (0.00148f64 / 4.0).sqrt() / 5f64.sqrt();
        assert!((get(&env, "uc") - (0.05f64.powi(2) / 3.0 + 0.0025 + 0.0004 + u4 * u4).sqrt()).abs() < 1e-9);
    }

    #[test]
    fn csv() {
        let rows = vec![vec!["a, b".into(), "1".into()], vec!["say \"hi\"".into(), "−2".into()]];
        assert_eq!(to_csv(&["line", "result"], &rows), "line,result\n\"a, b\",1\n\"say \"\"hi\"\"\",-2\n");
    }
}
