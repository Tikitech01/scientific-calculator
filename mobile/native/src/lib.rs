//! Small C ABI around the desktop calculator engine for Flutter.
#![allow(dead_code)] // The shared desktop modules expose more than this bridge needs.

#[path = "../../../src/engine.rs"]
mod engine;
#[path = "../../../src/finance.rs"]
mod finance;
#[path = "../../../src/fmt.rs"]
mod fmt;
#[path = "../../../src/hypothesis.rs"]
mod hypothesis;
#[path = "../../../src/matrix.rs"]
mod matrix;
#[path = "../../../src/numeric.rs"]
mod numeric;
#[path = "../../../src/sheet.rs"]
mod sheet;
#[path = "../../../src/stats.rs"]
mod stats;
#[path = "../../../src/units.rs"]
mod units;

use engine::{Angle, Env};
use serde_json::json;
use std::ffi::{c_char, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};

fn output(value: serde_json::Value) -> *mut c_char {
    CString::new(value.to_string()).unwrap().into_raw()
}

fn input(ptr: *const c_char) -> Result<String, String> {
    if ptr.is_null() {
        return Err("Missing expression".into());
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map(str::to_owned)
        .map_err(|_| "Expression must be UTF-8".into())
}

fn answer(result: Result<engine::Value, String>) -> serde_json::Value {
    match result {
        Ok(value) => json!({"ok": true, "value": fmt::plain(&value)}),
        Err(error) => json!({"ok": false, "error": error}),
    }
}

#[no_mangle]
pub extern "C" fn scicalc_new() -> *mut Env {
    Box::into_raw(Box::new(Env::default()))
}

#[no_mangle]
pub unsafe extern "C" fn scicalc_free(env: *mut Env) {
    if !env.is_null() {
        drop(Box::from_raw(env));
    }
}

#[no_mangle]
pub unsafe extern "C" fn scicalc_string_free(text: *mut c_char) {
    if !text.is_null() {
        drop(CString::from_raw(text));
    }
}

#[no_mangle]
pub unsafe extern "C" fn scicalc_set_angle(env: *mut Env, angle: u8) {
    if let Some(env) = env.as_mut() {
        env.angle = match angle {
            1 => Angle::Rad,
            2 => Angle::Grad,
            _ => Angle::Deg,
        };
    }
}

#[no_mangle]
pub extern "C" fn scicalc_loan_payment(principal: f64, rate: f64, months: f64) -> f64 {
    finance::emi(principal, rate, months)
}

#[no_mangle]
pub unsafe extern "C" fn scicalc_eval(env: *mut Env, text: *const c_char, commit: bool) -> *mut c_char {
    if env.is_null() {
        return output(json!({"ok": false, "error": "Calculator is closed"}));
    }
    let source = match input(text) {
        Ok(source) => source,
        Err(error) => return output(json!({"ok": false, "error": error})),
    };
    match catch_unwind(AssertUnwindSafe(|| {
        let env = &mut *env;
        if commit { engine::evaluate(&source, env) } else { engine::evaluate_pure(&source, env) }
    })) {
        Ok(result) => output(answer(result)),
        Err(_) => output(json!({"ok": false, "error": "Calculation failed"})),
    }
}

#[no_mangle]
pub unsafe extern "C" fn scicalc_eval_at(env: *mut Env, text: *const c_char, x: f64) -> f64 {
    if env.is_null() || !x.is_finite() {
        return f64::NAN;
    }
    let source = match input(text) {
        Ok(source) => source,
        Err(_) => return f64::NAN,
    };
    catch_unwind(AssertUnwindSafe(|| {
        let expr = engine::parse(&source).ok()?;
        engine::with_var(&mut *env, "x", |env| Some(engine::eval_at(&expr, env, "x", x)))
    }))
    .ok()
    .flatten()
    .unwrap_or(f64::NAN)
}

#[no_mangle]
pub unsafe extern "C" fn scicalc_sheet(env: *mut Env, text: *const c_char) -> *mut c_char {
    if env.is_null() {
        return output(json!({"ok": false, "error": "Calculator is closed"}));
    }
    let source = match input(text) {
        Ok(source) => source,
        Err(error) => return output(json!({"ok": false, "error": error})),
    };
    match catch_unwind(AssertUnwindSafe(|| sheet::run(&source, &mut *env))) {
        Ok(lines) => output(json!({"ok": true, "lines": lines.iter().map(|line| match line {
            sheet::Line::Nothing => json!({"kind": "empty"}),
            sheet::Line::Value(value) => json!({"kind": "value", "value": fmt::plain(value)}),
            sheet::Line::Error(error) => json!({"kind": "error", "value": error}),
        }).collect::<Vec<_>>()})),
        Err(_) => output(json!({"ok": false, "error": "Sheet calculation failed"})),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_pro_sheet_and_graph() {
        let mut env = Env::default();
        assert_eq!(fmt::plain(&engine::evaluate("240*18%", &mut env).unwrap()), "43.2");
        assert_eq!(fmt::plain(&engine::evaluate("sin(30)+sqrt(9)", &mut env).unwrap()), "3.5");
        let lines = sheet::run("R=4700\nC=100n\nfc=1/(2*pi*R*C)", &mut env);
        assert!(matches!(lines[2], sheet::Line::Value(_)));
        let expr = engine::parse("x^2").unwrap();
        assert_eq!(engine::with_var(&mut env, "x", |env| engine::eval_at(&expr, env, "x", 3.0)), 9.0);
        assert!((scicalc_loan_payment(200_000.0, 9.0, 240.0) - 1799.45).abs() < 0.01);
    }
}
