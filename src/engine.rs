//! Expression lexer, parser and evaluator.

use crate::matrix::Matrix;
use crate::{numeric, stats};
use num_complex::Complex64 as C;
use std::collections::HashMap;
use std::f64::consts::PI;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Angle {
    Deg,
    Rad,
    Grad,
}

impl Angle {
    pub fn to_rad(self, x: f64) -> f64 {
        match self {
            Angle::Deg => x * PI / 180.0,
            Angle::Rad => x,
            Angle::Grad => x * PI / 200.0,
        }
    }
    pub fn from_rad(self, x: f64) -> f64 {
        match self {
            Angle::Deg => x * 180.0 / PI,
            Angle::Rad => x,
            Angle::Grad => x * 200.0 / PI,
        }
    }
    pub fn full_turn(self) -> f64 {
        match self {
            Angle::Deg => 360.0,
            Angle::Rad => 2.0 * PI,
            Angle::Grad => 400.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Num(C),
    Mat(Matrix),
    List(Vec<f64>),
}

impl Value {
    pub fn real(x: f64) -> Value {
        Value::Num(C::new(x, 0.0))
    }
    pub fn as_real(&self) -> Option<f64> {
        match self {
            Value::Num(c) if c.im == 0.0 => Some(c.re),
            _ => None,
        }
    }
}

pub struct Env {
    pub vars: HashMap<String, Value>,
    pub angle: Angle,
    pub ans: Value,
}

impl Default for Env {
    fn default() -> Self {
        Env { vars: HashMap::new(), angle: Angle::Deg, ans: Value::real(0.0) }
    }
}

impl Env {
    /// The last answer when it is a real number.
    pub fn ans_real(&self) -> Option<f64> {
        match &self.ans {
            Value::Num(z) if z.im == 0.0 => Some(z.re),
            _ => None,
        }
    }

    pub fn set_real(&mut self, name: &str, x: f64) {
        if let Some(v) = self.vars.get_mut(name) {
            *v = Value::real(x);
        } else {
            self.vars.insert(name.to_string(), Value::real(x));
        }
    }
}

#[derive(Clone, Debug)]
pub enum Expr {
    Num(f64),
    Var(String),
    Neg(Box<Expr>),
    Bin(char, Box<Expr>, Box<Expr>),
    Post(char, Box<Expr>),
    Call(String, Vec<Expr>),
    Mat(Vec<Vec<Expr>>),
    List(Vec<Expr>),
}

pub const FUNCS: &[&str] = &[
    "sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "asinh", "acosh", "atanh", "ln", "log",
    "log2", "exp", "sqrt", "cbrt", "root", "abs", "arg", "conj", "re", "im", "int", "frac", "floor", "ceil",
    "round", "sign", "fact", "gamma", "nPr", "nCr", "gcd", "lcm", "mod", "min", "max", "rand", "randint", "sum",
    "prod", "mean", "median", "stdev", "stdevp", "variance", "len", "sort", "det", "inv", "trn", "rref", "rank",
    "trace", "identity", "dim", "norm", "dot", "cross", "Pol", "Rec", "and", "or", "xor", "not", "shl", "shr",
    "normpdf", "normcdf", "invnorm", "tpdf", "tcdf", "invt", "chi2pdf", "chi2cdf", "invchi2", "fpdf", "fcdf",
    "invf", "binompdf", "binomcdf", "poissonpdf", "poissoncdf", "diff", "integ", "solve",
    "seq",
];

const NAMED: &[&str] = &[
    "pi", "theta", "Ans", "MatAns", "MatA", "MatB", "MatC", "MatD", "MatE", "MatF", "List1", "List2", "List3", "List4",
    "List5", "List6",
];

/// Short function names that only count as functions when directly followed by "(".
const NEEDS_PAREN: &[&str] = &["re", "im", "or", "and", "not", "int", "len", "dim", "min", "max", "mod", "exp"];

pub fn is_function(name: &str) -> bool {
    FUNCS.contains(&name)
}

// ---------------------------------------------------------------- lexer

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    /// A raised number such as ² or ⁻¹⁰.
    Sup(f64),
    Id(String),
    Sym(char),
}

fn sup_digit(c: char) -> Option<u32> {
    Some(match c {
        '⁰' => 0,
        '¹' => 1,
        '²' => 2,
        '³' => 3,
        '⁴' => 4,
        '⁵' => 5,
        '⁶' => 6,
        '⁷' => 7,
        '⁸' => 8,
        '⁹' => 9,
        _ => return None,
    })
}

fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let ch: Vec<char> = src.chars().collect();
    let n = ch.len();
    let mut i = 0;
    let mut out = Vec::new();
    while i < n {
        let c = ch[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && i + 1 < n && ch[i + 1].is_ascii_digit()) {
            // Base-prefixed integers.
            if c == '0' && i + 2 < n {
                let radix = match ch[i + 1] {
                    'x' => 16,
                    'b' => 2,
                    'o' => 8,
                    _ => 0,
                };
                if radix != 0 && ch[i + 2].is_digit(radix) {
                    let mut j = i + 2;
                    let mut s = String::new();
                    while j < n && ch[j].is_digit(radix) {
                        s.push(ch[j]);
                        j += 1;
                    }
                    let v = u64::from_str_radix(&s, radix).map_err(|_| "Number too large".to_string())?;
                    out.push(Tok::Num(v as i64 as f64));
                    i = j;
                    continue;
                }
            }
            let mut s = String::new();
            while i < n && ch[i].is_ascii_digit() {
                s.push(ch[i]);
                i += 1;
            }
            if i < n && ch[i] == '.' {
                s.push('.');
                i += 1;
                while i < n && ch[i].is_ascii_digit() {
                    s.push(ch[i]);
                    i += 1;
                }
            }
            // Exponent: 1.5E3, 1.5E-3, 1e3
            if i < n && (ch[i] == 'E' || ch[i] == 'e') {
                let upper = ch[i] == 'E';
                let mut j = i + 1;
                let mut e = String::new();
                if upper && j < n && (ch[j] == '-' || ch[j] == '−' || ch[j] == '+') {
                    e.push(if ch[j] == '+' { '+' } else { '-' });
                    j += 1;
                }
                if j < n && ch[j].is_ascii_digit() {
                    while j < n && ch[j].is_ascii_digit() {
                        e.push(ch[j]);
                        j += 1;
                    }
                    s.push('e');
                    s.push_str(&e);
                    i = j;
                }
            }
            let v: f64 = s.parse().map_err(|_| format!("Bad number “{}”", s))?;
            out.push(Tok::Num(v));
            continue;
        }
        if c.is_alphabetic() {
            if c == 'π' {
                out.push(Tok::Id("pi".into()));
                i += 1;
                continue;
            }
            // Longest known name starting here.
            let mut best: Option<&str> = None;
            for name in FUNCS.iter().chain(NAMED.iter()) {
                let len = name.chars().count();
                if i + len <= n && name.chars().zip(&ch[i..i + len]).all(|(a, b)| a == *b) {
                    if NEEDS_PAREN.contains(name) {
                        let mut j = i + len;
                        while j < n && ch[j] == ' ' {
                            j += 1;
                        }
                        if !(j < n && ch[j] == '(') {
                            continue;
                        }
                    }
                    if best.map(|b| b.len() < name.len()).unwrap_or(true) {
                        best = Some(name);
                    }
                }
            }
            match best {
                Some(name) => {
                    out.push(Tok::Id(if name == "theta" { "θ".to_string() } else { name.to_string() }));
                    i += name.chars().count();
                }
                None => {
                    out.push(Tok::Id(c.to_string()));
                    i += 1;
                }
            }
            continue;
        }
        if c == 'θ' {
            out.push(Tok::Id("θ".into()));
            i += 1;
            continue;
        }
        if c == '⁻' || sup_digit(c).is_some() {
            let neg = c == '⁻';
            let mut j = if neg { i + 1 } else { i };
            let mut v = 0.0f64;
            let start = j;
            while j < n {
                match sup_digit(ch[j]) {
                    Some(d) => v = v * 10.0 + d as f64,
                    None => break,
                }
                j += 1;
            }
            if j == start {
                return Err("Unexpected “⁻”".into());
            }
            out.push(Tok::Sup(if neg { -v } else { v }));
            i = j;
            continue;
        }
        let sym = match c {
            '+' => '+',
            '-' | '−' | '–' => '-',
            '*' | '×' | '·' | '⋅' => '*',
            '/' | '÷' => '/',
            '^' | '!' | '%' | '(' | ')' | ',' | '[' | ']' | '{' | '}' | ';' | '°' | '√' | '∠' => c,
            '=' => return Err("“=” is not allowed here".into()),
            _ => return Err(format!("Unexpected “{}”", c)),
        };
        out.push(Tok::Sym(sym));
        i += 1;
    }
    Ok(out)
}

// ---------------------------------------------------------------- parser

struct Parser {
    t: Vec<Tok>,
    p: usize,
}

type PResult = Result<Expr, String>;

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.p)
    }
    fn is(&self, c: char) -> bool {
        matches!(self.peek(), Some(Tok::Sym(s)) if *s == c)
    }
    fn eat(&mut self, c: char) -> bool {
        if self.is(c) {
            self.p += 1;
            true
        } else {
            false
        }
    }
    /// Closing brackets may be omitted at the very end of the input.
    fn close(&mut self, c: char) -> Result<(), String> {
        if self.eat(c) || self.peek().is_none() {
            Ok(())
        } else {
            Err(format!("Expected “{}”", c))
        }
    }

    fn expr(&mut self) -> PResult {
        let mut lhs = self.term()?;
        loop {
            if self.eat('+') {
                lhs = Expr::Bin('+', Box::new(lhs), Box::new(self.term()?));
            } else if self.eat('-') {
                lhs = Expr::Bin('-', Box::new(lhs), Box::new(self.term()?));
            } else {
                return Ok(lhs);
            }
        }
    }

    fn term(&mut self) -> PResult {
        let mut lhs = self.implicit()?;
        loop {
            if self.eat('*') {
                lhs = Expr::Bin('*', Box::new(lhs), Box::new(self.implicit()?));
            } else if self.eat('/') {
                lhs = Expr::Bin('/', Box::new(lhs), Box::new(self.implicit()?));
            } else {
                return Ok(lhs);
            }
        }
    }

    fn starts_operand(&self) -> bool {
        match self.peek() {
            Some(Tok::Num(_)) | Some(Tok::Id(_)) => true,
            Some(Tok::Sym(c)) => matches!(c, '(' | '√' | '[' | '{'),
            _ => false,
        }
    }

    /// Implicit multiplication binds tighter than × and ÷ (so 1÷2π = 1÷(2π)).
    fn implicit(&mut self) -> PResult {
        let mut lhs = self.unary()?;
        while self.starts_operand() {
            let rhs = self.power()?;
            lhs = Expr::Bin('*', Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> PResult {
        if self.eat('-') {
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        if self.eat('+') {
            return self.unary();
        }
        self.power()
    }

    fn power(&mut self) -> PResult {
        let mut base = self.postfix()?;
        // Polar form r∠θ, with θ in the current angle unit.
        if self.eat('∠') {
            let ang = self.postfix()?;
            base = Expr::Bin('∠', Box::new(base), Box::new(ang));
        }
        if self.eat('^') {
            let exp = self.unary()?;
            return Ok(Expr::Bin('^', Box::new(base), Box::new(exp)));
        }
        Ok(base)
    }

    fn postfix(&mut self) -> PResult {
        let mut e = self.primary()?;
        loop {
            if let Some(Tok::Sup(v)) = self.peek() {
                let v = *v;
                self.p += 1;
                e = Expr::Bin('^', Box::new(e), Box::new(Expr::Num(v)));
                continue;
            }
            let c = match self.peek() {
                Some(Tok::Sym(c)) if matches!(c, '!' | '%' | '°') => *c,
                _ => return Ok(e),
            };
            self.p += 1;
            e = Expr::Post(c, Box::new(e));
        }
    }

    fn args(&mut self, close: char) -> Result<Vec<Expr>, String> {
        let mut v = Vec::new();
        if self.eat(close) {
            return Ok(v);
        }
        loop {
            v.push(self.expr()?);
            if self.eat(',') {
                continue;
            }
            self.close(close)?;
            return Ok(v);
        }
    }

    fn primary(&mut self) -> PResult {
        let tok = self.peek().cloned().ok_or_else(|| "Incomplete expression".to_string())?;
        self.p += 1;
        match tok {
            Tok::Num(v) => Ok(Expr::Num(v)),
            Tok::Id(name) => {
                if is_function(&name) {
                    if !self.eat('(') {
                        return Err(format!("Use parentheses: {}(…)", name));
                    }
                    let a = self.args(')')?;
                    Ok(Expr::Call(name, a))
                } else {
                    Ok(Expr::Var(name))
                }
            }
            Tok::Sym('(') => {
                let e = self.expr()?;
                self.close(')')?;
                Ok(e)
            }
            Tok::Sym('√') => {
                let e = self.power()?;
                Ok(Expr::Call("sqrt".into(), vec![e]))
            }
            Tok::Sym('{') => Ok(Expr::List(self.args('}')?)),
            Tok::Sym('[') => {
                let mut rows = Vec::new();
                if self.is('[') {
                    // [[1,2],[3,4]]
                    loop {
                        if !self.eat('[') {
                            return Err("Expected “[”".into());
                        }
                        rows.push(self.args(']')?);
                        if self.eat(',') {
                            continue;
                        }
                        self.close(']')?;
                        break;
                    }
                } else {
                    // [1,2;3,4]
                    let mut row = Vec::new();
                    loop {
                        row.push(self.expr()?);
                        if self.eat(',') {
                            continue;
                        }
                        if self.eat(';') {
                            rows.push(std::mem::take(&mut row));
                            continue;
                        }
                        self.close(']')?;
                        rows.push(row);
                        break;
                    }
                }
                Ok(Expr::Mat(rows))
            }
            Tok::Sym(c) => Err(format!("Unexpected “{}”", if c == '-' { '−' } else { c })),
            Tok::Sup(_) => Err("A raised number needs something before it".into()),
        }
    }
}

pub fn parse(src: &str) -> PResult {
    let t = lex(src)?;
    if t.is_empty() {
        return Err("Empty expression".into());
    }
    let mut p = Parser { t, p: 0 };
    let e = p.expr()?;
    if p.p < p.t.len() {
        return Err(match &p.t[p.p] {
            Tok::Sym(c) => format!("Unexpected “{}”", c),
            Tok::Num(_) | Tok::Sup(_) => "Unexpected number".into(),
            Tok::Id(s) => format!("Unexpected “{}”", s),
        });
    }
    Ok(e)
}

/// Every variable name used in an expression, in order of first appearance.
pub fn var_names(e: &Expr, out: &mut Vec<String>) {
    match e {
        Expr::Num(_) => {}
        Expr::Var(v) => {
            if !out.contains(v) {
                out.push(v.clone());
            }
        }
        Expr::Neg(a) | Expr::Post(_, a) => var_names(a, out),
        Expr::Bin(_, a, b) => {
            var_names(a, out);
            var_names(b, out);
        }
        Expr::Call(_, a) | Expr::List(a) => a.iter().for_each(|x| var_names(x, out)),
        Expr::Mat(rows) => rows.iter().flatten().for_each(|x| var_names(x, out)),
    }
}

pub fn uses_var(e: &Expr, name: &str) -> bool {
    match e {
        Expr::Num(_) => false,
        Expr::Var(v) => v == name,
        Expr::Neg(a) | Expr::Post(_, a) => uses_var(a, name),
        Expr::Bin(_, a, b) => uses_var(a, name) || uses_var(b, name),
        Expr::Call(_, a) | Expr::List(a) => a.iter().any(|x| uses_var(x, name)),
        Expr::Mat(rows) => rows.iter().flatten().any(|x| uses_var(x, name)),
    }
}

// ---------------------------------------------------------------- evaluation

type R = Result<Value, String>;

fn clean(mut z: C) -> C {
    if z.im != 0.0 && z.im.abs() <= 1e-13 * z.re.abs() {
        z.im = 0.0;
    }
    if z.re != 0.0 && z.re.abs() <= 1e-13 * z.im.abs() {
        z.re = 0.0;
    }
    z
}

fn snap(v: f64) -> f64 {
    if v.abs() < 1e-15 {
        0.0
    } else {
        v
    }
}

fn real_of(v: &Value) -> Result<f64, String> {
    match v {
        Value::Num(c) if c.im == 0.0 => Ok(c.re),
        Value::Num(_) => Err("A real number is required".into()),
        _ => Err("A number is required".into()),
    }
}

fn int_of(v: &Value) -> Result<i64, String> {
    let x = real_of(v)?;
    if x.fract() != 0.0 || x.abs() > 9.0e18 {
        return Err("A whole number is required".into());
    }
    Ok(x as i64)
}

fn list_of(v: &Value) -> Result<Vec<f64>, String> {
    match v {
        Value::List(l) => Ok(l.clone()),
        Value::Mat(m) if m.r == 1 || m.c == 1 => Ok(m.d.clone()),
        _ => Err("A list is required".into()),
    }
}

fn mat_of(v: &Value) -> Result<&Matrix, String> {
    match v {
        Value::Mat(m) => Ok(m),
        _ => Err("A matrix is required".into()),
    }
}

fn num_bin(op: char, x: C, y: C) -> Result<C, String> {
    let real = x.im == 0.0 && y.im == 0.0;
    let (a, b) = (x.re, y.re);
    Ok(match op {
        '+' => {
            if real {
                C::new(a + b, 0.0)
            } else {
                x + y
            }
        }
        '-' => {
            if real {
                C::new(a - b, 0.0)
            } else {
                x - y
            }
        }
        '*' => {
            if real {
                C::new(a * b, 0.0)
            } else {
                clean(x * y)
            }
        }
        '/' => {
            if y.re == 0.0 && y.im == 0.0 {
                return Err("Division by zero".into());
            }
            if real {
                C::new(a / b, 0.0)
            } else {
                clean(x / y)
            }
        }
        '^' => {
            if real {
                if a == 0.0 && b < 0.0 {
                    return Err("Division by zero".into());
                }
                if a == 0.0 && b == 0.0 {
                    return Err("0^0 is undefined".into());
                }
                if a >= 0.0 || b.fract() == 0.0 {
                    let v = if b.fract() == 0.0 && b.abs() < 1e9 { a.powi(b as i32) } else { a.powf(b) };
                    return Ok(C::new(v, 0.0));
                }
            }
            if x.re == 0.0 && x.im == 0.0 {
                if y.re > 0.0 {
                    C::new(0.0, 0.0)
                } else {
                    return Err("Division by zero".into());
                }
            } else if y.im == 0.0 && y.re.fract() == 0.0 && y.re.abs() < 1e6 {
                clean(x.powi(y.re as i32))
            } else {
                clean(x.powc(y))
            }
        }
        _ => return Err("Unknown operator".into()),
    })
}

fn real_bin(op: char, a: f64, b: f64) -> Result<f64, String> {
    let z = num_bin(op, C::new(a, 0.0), C::new(b, 0.0))?;
    if z.im != 0.0 {
        return Err("Non-real result in a list".into());
    }
    Ok(z.re)
}

fn bin(op: char, a: Value, b: Value) -> R {
    use Value::*;
    match (a, b) {
        (Num(x), Num(y)) => num_bin(op, x, y).map(Num),
        (Mat(x), Mat(y)) => match op {
            '+' => x.add(&y).map(Mat),
            '-' => x.sub(&y).map(Mat),
            '*' => x.mul(&y).map(Mat),
            '/' => x.mul(&y.inverse()?).map(Mat),
            _ => Err("That operation is not defined for two matrices".into()),
        },
        (Mat(m), Num(s)) => {
            if s.im != 0.0 {
                return Err("Matrices work with real numbers only".into());
            }
            match op {
                '*' => Ok(Mat(m.scale(s.re))),
                '/' => {
                    if s.re == 0.0 {
                        Err("Division by zero".into())
                    } else {
                        Ok(Mat(m.scale(1.0 / s.re)))
                    }
                }
                '^' => {
                    if s.re.fract() != 0.0 {
                        return Err("Matrix powers must be whole numbers".into());
                    }
                    m.pow(s.re as i64).map(Mat)
                }
                _ => Err("Cannot add a number to a matrix".into()),
            }
        }
        (Num(s), Mat(m)) => {
            if s.im != 0.0 {
                return Err("Matrices work with real numbers only".into());
            }
            match op {
                '*' => Ok(Mat(m.scale(s.re))),
                _ => Err("That operation is not defined for a number and a matrix".into()),
            }
        }
        (List(x), List(y)) => {
            if x.len() != y.len() {
                return Err("Lists must have the same length".into());
            }
            let mut o = Vec::with_capacity(x.len());
            for (p, q) in x.iter().zip(&y) {
                o.push(real_bin(op, *p, *q)?);
            }
            Ok(List(o))
        }
        (List(x), Num(s)) => {
            let s = real_of(&Num(s))?;
            x.iter().map(|p| real_bin(op, *p, s)).collect::<Result<Vec<_>, _>>().map(List)
        }
        (Num(s), List(x)) => {
            let s = real_of(&Num(s))?;
            x.iter().map(|p| real_bin(op, s, *p)).collect::<Result<Vec<_>, _>>().map(List)
        }
        _ => Err("Those types cannot be combined".into()),
    }
}

fn map1(v: &Value, f: impl Fn(C) -> Result<C, String>) -> R {
    match v {
        Value::Num(z) => Ok(Value::Num(clean(f(*z)?))),
        Value::List(l) => {
            let mut o = Vec::with_capacity(l.len());
            for x in l {
                let r = clean(f(C::new(*x, 0.0))?);
                if r.im != 0.0 {
                    return Err("Non-real result in a list".into());
                }
                o.push(r.re);
            }
            Ok(Value::List(o))
        }
        Value::Mat(_) => Err("That function is not defined for matrices".into()),
    }
}

fn map_real(v: &Value, f: impl Fn(f64) -> Result<f64, String>) -> R {
    map1(v, |z| {
        if z.im != 0.0 {
            return Err("A real number is required".into());
        }
        f(z.re).map(|r| C::new(r, 0.0))
    })
}

fn trig(name: &str, z: C, ang: Angle) -> Result<C, String> {
    if z.im == 0.0 {
        let x = z.re % ang.full_turn();
        let t = ang.to_rad(x);
        let (s, c) = (snap(t.sin()), snap(t.cos()));
        let r = match name {
            "sin" => s,
            "cos" => c,
            _ => {
                if c == 0.0 {
                    return Err("tan is undefined at this angle".into());
                }
                s / c
            }
        };
        Ok(C::new(r, 0.0))
    } else {
        Ok(match name {
            "sin" => z.sin(),
            "cos" => z.cos(),
            _ => z.tan(),
        })
    }
}

fn atrig(name: &str, z: C, ang: Angle) -> Result<C, String> {
    if z.im == 0.0 {
        let x = z.re;
        let r = match name {
            "asin" if x.abs() <= 1.0 => Some(x.asin()),
            "acos" if x.abs() <= 1.0 => Some(x.acos()),
            "atan" => Some(x.atan()),
            _ => None,
        };
        if let Some(r) = r {
            return Ok(C::new(ang.from_rad(r), 0.0));
        }
    }
    Ok(match name {
        "asin" => z.asin(),
        "acos" => z.acos(),
        _ => z.atan(),
    })
}

pub fn factorial(x: f64) -> Result<f64, String> {
    if x < 0.0 && x.fract() == 0.0 {
        return Err("Factorial of a negative whole number is undefined".into());
    }
    if x.fract() == 0.0 {
        if x > 170.0 {
            return Err("Result is too large".into());
        }
        let mut r = 1.0;
        let mut k = 2.0;
        while k <= x {
            r *= k;
            k += 1.0;
        }
        Ok(r)
    } else {
        let g = libm::tgamma(x + 1.0);
        if g.is_finite() {
            Ok(g)
        } else {
            Err("Result is too large".into())
        }
    }
}

fn n_c_r(n: f64, r: f64, perm: bool) -> Result<f64, String> {
    if n.fract() != 0.0 || r.fract() != 0.0 || n < 0.0 || r < 0.0 {
        return Err("nCr and nPr need whole numbers ≥ 0".into());
    }
    if r > n {
        return Ok(0.0);
    }
    let mut acc = 1.0f64;
    if perm {
        let mut k = 0.0;
        while k < r {
            acc *= n - k;
            k += 1.0;
        }
    } else {
        let r = r.min(n - r);
        let mut k = 1.0;
        while k <= r {
            acc = acc * (n - r + k) / k;
            k += 1.0;
        }
        acc = acc.round();
    }
    if acc.is_finite() {
        Ok(acc)
    } else {
        Err("Result is too large".into())
    }
}

fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

fn next_random() -> f64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static STATE: AtomicU64 = AtomicU64::new(0);
    let mut s = STATE.load(Ordering::Relaxed);
    if s == 0 {
        s = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E3779B97F4A7C15)
            | 1;
    }
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    STATE.store(s, Ordering::Relaxed);
    (s >> 11) as f64 / (1u64 << 53) as f64
}

pub fn eval_at(e: &Expr, env: &mut Env, var: &str, x: f64) -> f64 {
    env.set_real(var, x);
    match eval(e, env) {
        Ok(Value::Num(c)) if c.im == 0.0 => c.re,
        _ => f64::NAN,
    }
}

/// Run `f` with `var` temporarily free to be rebound, restoring it afterwards.
pub fn with_var<T>(env: &mut Env, var: &str, f: impl FnOnce(&mut Env) -> T) -> T {
    let old = env.vars.remove(var);
    let r = f(env);
    match old {
        Some(v) => {
            env.vars.insert(var.to_string(), v);
        }
        None => {
            env.vars.remove(var);
        }
    }
    r
}

fn split_var<'a>(args: &'a [Expr], k: usize) -> Result<(&'a Expr, String, &'a [Expr]), String> {
    if args.len() == k + 1 {
        Ok((&args[0], "x".into(), &args[1..]))
    } else if args.len() == k + 2 {
        match &args[1] {
            Expr::Var(v) => Ok((&args[0], v.clone(), &args[2..])),
            _ => Err("Expected a variable name as the second argument".into()),
        }
    } else {
        Err("Wrong number of arguments".into())
    }
}

fn special(name: &str, args: &[Expr], env: &mut Env) -> Option<R> {
    let is_special = match name {
        "diff" | "integ" | "solve" | "seq" => true,
        "sum" | "prod" => args.len() == 4,
        _ => false,
    };
    if !is_special {
        return None;
    }
    Some((|| -> R {
        let num = |e: &Expr, env: &mut Env| -> Result<f64, String> { real_of(&eval(e, env)?) };
        match name {
            "diff" => {
                let (body, var, rest) = split_var(args, 1)?;
                let a = num(&rest[0], env)?;
                let v = with_var(env, &var, |env| numeric::derivative(&mut |x| eval_at(body, env, &var, x), a));
                finite(v, "Cannot differentiate here")
            }
            "integ" => {
                let (body, var, rest) = split_var(args, 2)?;
                let a = num(&rest[0], env)?;
                let b = num(&rest[1], env)?;
                let v = with_var(env, &var, |env| numeric::integrate(&mut |x| eval_at(body, env, &var, x), a, b));
                finite(v, "The integral does not converge")
            }
            "solve" => {
                let (body, var, guess) = if args.len() == 1 {
                    (&args[0], "x".to_string(), 0.0)
                } else {
                    let (body, var, rest) = split_var(args, 1)?;
                    (body, var, num(&rest[0], env)?)
                };
                let v = with_var(env, &var, |env| numeric::solve_near(&mut |x| eval_at(body, env, &var, x), guess));
                v.map(Value::real).ok_or_else(|| "No solution found".to_string())
            }
            "sum" | "prod" => {
                let var = match &args[1] {
                    Expr::Var(v) => v.clone(),
                    _ => return Err("Expected a variable name as the second argument".into()),
                };
                let a = int_of(&eval(&args[2], env)?)?;
                let b = int_of(&eval(&args[3], env)?)?;
                if b - a > 1_000_000 {
                    return Err("Too many terms".into());
                }
                let body = &args[0];
                with_var(env, &var, |env| {
                    let mut acc = C::new(if name == "sum" { 0.0 } else { 1.0 }, 0.0);
                    for k in a..=b {
                        env.set_real(&var, k as f64);
                        let term = match eval(body, env)? {
                            Value::Num(z) => z,
                            _ => return Err("Each term must be a number".into()),
                        };
                        acc = num_bin(if name == "sum" { '+' } else { '*' }, acc, term)?;
                    }
                    Ok(Value::Num(acc))
                })
            }
            "seq" => {
                if args.len() < 4 || args.len() > 5 {
                    return Err("seq(expression, variable, start, end, step)".into());
                }
                let var = match &args[1] {
                    Expr::Var(v) => v.clone(),
                    _ => return Err("Expected a variable name as the second argument".into()),
                };
                let a = num(&args[2], env)?;
                let b = num(&args[3], env)?;
                let step = if args.len() == 5 { num(&args[4], env)? } else { 1.0 };
                if step == 0.0 || ((b - a) / step) > 100_000.0 {
                    return Err("Bad step".into());
                }
                let body = &args[0];
                with_var(env, &var, |env| {
                    let mut out = Vec::new();
                    let count = ((b - a) / step + 1e-9).floor();
                    let mut k = 0.0;
                    while k <= count {
                        env.set_real(&var, a + k * step);
                        out.push(real_of(&eval(body, env)?)?);
                        k += 1.0;
                    }
                    Ok(Value::List(out))
                })
            }
            _ => unreachable!(),
        }
    })())
}

fn finite(v: f64, msg: &str) -> R {
    if v.is_finite() {
        Ok(Value::real(v))
    } else {
        Err(msg.into())
    }
}

fn call(name: &str, a: &[Value], ang: Angle) -> R {
    let n = a.len();
    let need = |lo: usize, hi: usize| -> Result<(), String> {
        if n >= lo && n <= hi {
            Ok(())
        } else if lo == hi {
            Err(format!("{}() takes {} argument{}", name, lo, if lo == 1 { "" } else { "s" }))
        } else {
            Err(format!("{}() takes {} to {} arguments", name, lo, hi))
        }
    };
    let r = |i: usize| real_of(&a[i]);
    match name {
        "sin" | "cos" | "tan" => {
            need(1, 1)?;
            map1(&a[0], |z| trig(name, z, ang))
        }
        "asin" | "acos" | "atan" => {
            need(1, 1)?;
            map1(&a[0], |z| atrig(name, z, ang))
        }
        "sinh" => need(1, 1).and_then(|_| map1(&a[0], |z| Ok(z.sinh()))),
        "cosh" => need(1, 1).and_then(|_| map1(&a[0], |z| Ok(z.cosh()))),
        "tanh" => need(1, 1).and_then(|_| map1(&a[0], |z| Ok(z.tanh()))),
        "asinh" => need(1, 1).and_then(|_| map1(&a[0], |z| Ok(z.asinh()))),
        "acosh" => need(1, 1).and_then(|_| map1(&a[0], |z| Ok(z.acosh()))),
        "atanh" => need(1, 1).and_then(|_| {
            map1(&a[0], |z| {
                if z.im == 0.0 && z.re.abs() == 1.0 {
                    Err("atanh is undefined at ±1".into())
                } else if z.im == 0.0 && z.re.abs() < 1.0 {
                    Ok(C::new(z.re.atanh(), 0.0))
                } else {
                    Ok(z.atanh())
                }
            })
        }),
        "ln" | "log2" => {
            need(1, 1)?;
            let base = if name == "ln" { std::f64::consts::E } else { 2.0 };
            map1(&a[0], |z| log_base(z, base))
        }
        "log" => {
            need(1, 2)?;
            if n == 1 {
                map1(&a[0], |z| log_base(z, 10.0))
            } else {
                let b = r(0)?;
                if b <= 0.0 || b == 1.0 {
                    return Err("The log base must be positive and not 1".into());
                }
                map1(&a[1], |z| log_base(z, b))
            }
        }
        "exp" => need(1, 1).and_then(|_| map1(&a[0], |z| Ok(z.exp()))),
        "sqrt" => need(1, 1).and_then(|_| {
            map1(&a[0], |z| Ok(if z.im == 0.0 && z.re >= 0.0 { C::new(z.re.sqrt(), 0.0) } else { z.sqrt() }))
        }),
        "cbrt" => need(1, 1).and_then(|_| {
            map1(&a[0], |z| Ok(if z.im == 0.0 { C::new(z.re.cbrt(), 0.0) } else { z.powf(1.0 / 3.0) }))
        }),
        "root" => {
            need(2, 2)?;
            let k = r(0)?;
            if k == 0.0 {
                return Err("The root index cannot be 0".into());
            }
            map1(&a[1], |z| {
                if z.im == 0.0 && z.re < 0.0 && k.fract() == 0.0 && (k as i64) % 2 != 0 {
                    Ok(C::new(-(-z.re).powf(1.0 / k), 0.0))
                } else {
                    num_bin('^', z, C::new(1.0 / k, 0.0))
                }
            })
        }
        "abs" => {
            need(1, 1)?;
            match &a[0] {
                Value::Mat(m) => m.det().map(Value::real),
                v => map1(v, |z| Ok(C::new(z.norm(), 0.0))),
            }
        }
        "arg" => need(1, 1).and_then(|_| map1(&a[0], |z| Ok(C::new(ang.from_rad(z.arg()), 0.0)))),
        "conj" => need(1, 1).and_then(|_| map1(&a[0], |z| Ok(z.conj()))),
        "re" => need(1, 1).and_then(|_| map1(&a[0], |z| Ok(C::new(z.re, 0.0)))),
        "im" => need(1, 1).and_then(|_| map1(&a[0], |z| Ok(C::new(z.im, 0.0)))),
        "int" => need(1, 1).and_then(|_| map_real(&a[0], |x| Ok(x.trunc()))),
        "frac" => need(1, 1).and_then(|_| map_real(&a[0], |x| Ok(x.fract()))),
        "floor" => need(1, 1).and_then(|_| map_real(&a[0], |x| Ok(x.floor()))),
        "ceil" => need(1, 1).and_then(|_| map_real(&a[0], |x| Ok(x.ceil()))),
        "sign" => need(1, 1).and_then(|_| map_real(&a[0], |x| Ok(if x == 0.0 { 0.0 } else { x.signum() }))),
        "round" => {
            need(1, 2)?;
            let d = if n == 2 { int_of(&a[1])? } else { 0 };
            if !(-15..=15).contains(&d) {
                return Err("round() supports −15 to 15 places".into());
            }
            let p = 10f64.powi(d as i32);
            map_real(&a[0], |x| Ok((x * p).round() / p))
        }
        "fact" => need(1, 1).and_then(|_| map_real(&a[0], factorial)),
        "gamma" => need(1, 1).and_then(|_| {
            map_real(&a[0], |x| {
                let g = libm::tgamma(x);
                if g.is_finite() {
                    Ok(g)
                } else {
                    Err("gamma is undefined here".into())
                }
            })
        }),
        "nCr" | "nPr" => {
            need(2, 2)?;
            n_c_r(r(0)?, r(1)?, name == "nPr").map(Value::real)
        }
        "gcd" | "lcm" => {
            need(2, 8)?;
            let mut acc = int_of(&a[0])?;
            for v in &a[1..] {
                let b = int_of(v)?;
                acc = if name == "gcd" {
                    gcd(acc, b)
                } else if acc == 0 || b == 0 {
                    0
                } else {
                    (acc / gcd(acc, b)).checked_mul(b).ok_or("Result is too large")?.abs()
                };
            }
            Ok(Value::real(acc as f64))
        }
        "mod" => {
            need(2, 2)?;
            let (x, y) = (r(0)?, r(1)?);
            if y == 0.0 {
                return Err("Division by zero".into());
            }
            Ok(Value::real(x - y * (x / y).floor()))
        }
        "min" | "max" => {
            let vals: Vec<f64> = if n == 1 {
                list_of(&a[0])?
            } else {
                a.iter().map(real_of).collect::<Result<_, _>>()?
            };
            if vals.is_empty() {
                return Err("No values".into());
            }
            let v = if name == "min" {
                vals.iter().cloned().fold(f64::INFINITY, f64::min)
            } else {
                vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
            };
            Ok(Value::real(v))
        }
        "rand" => {
            need(0, 0)?;
            Ok(Value::real(next_random()))
        }
        "randint" => {
            need(2, 2)?;
            let (lo, hi) = (int_of(&a[0])?, int_of(&a[1])?);
            if hi < lo {
                return Err("randint(low, high) needs low ≤ high".into());
            }
            Ok(Value::real((lo + (next_random() * (hi - lo + 1) as f64).floor() as i64) as f64))
        }
        "sum" | "prod" | "mean" | "median" | "stdev" | "stdevp" | "variance" | "len" | "sort" => {
            need(1, 1)?;
            let mut l = list_of(&a[0])?;
            if l.is_empty() && name != "len" {
                return Err("The list is empty".into());
            }
            let cnt = l.len() as f64;
            let mean = l.iter().sum::<f64>() / cnt;
            let ss: f64 = l.iter().map(|x| (x - mean).powi(2)).sum();
            Ok(match name {
                "sum" => Value::real(l.iter().sum()),
                "prod" => Value::real(l.iter().product()),
                "mean" => Value::real(mean),
                "median" => {
                    l.sort_by(|p, q| p.partial_cmp(q).unwrap());
                    Value::real(stats::median_sorted(&l))
                }
                "stdev" | "variance" => {
                    if l.len() < 2 {
                        return Err("Need at least two values".into());
                    }
                    let var = ss / (cnt - 1.0);
                    Value::real(if name == "stdev" { var.sqrt() } else { var })
                }
                "stdevp" => Value::real((ss / cnt).sqrt()),
                "len" => Value::real(cnt),
                _ => {
                    l.sort_by(|p, q| p.partial_cmp(q).unwrap());
                    Value::List(l)
                }
            })
        }
        "det" => need(1, 1).and_then(|_| mat_of(&a[0])?.det().map(Value::real)),
        "inv" => {
            need(1, 1)?;
            match &a[0] {
                Value::Mat(m) => m.inverse().map(Value::Mat),
                v => bin('/', Value::real(1.0), v.clone()),
            }
        }
        "trn" => need(1, 1).and_then(|_| Ok(Value::Mat(mat_of(&a[0])?.transpose()))),
        "rref" => need(1, 1).and_then(|_| Ok(Value::Mat(mat_of(&a[0])?.rref().0))),
        "rank" => need(1, 1).and_then(|_| Ok(Value::real(mat_of(&a[0])?.rref().1 as f64))),
        "trace" => need(1, 1).and_then(|_| mat_of(&a[0])?.trace().map(Value::real)),
        "norm" => {
            need(1, 1)?;
            match &a[0] {
                Value::Mat(m) => Ok(Value::real(m.norm())),
                Value::List(l) => Ok(Value::real(l.iter().map(|v| v * v).sum::<f64>().sqrt())),
                Value::Num(z) => Ok(Value::real(z.norm())),
            }
        }
        "identity" => {
            need(1, 1)?;
            let k = int_of(&a[0])?;
            if !(1..=20).contains(&k) {
                return Err("identity(n) supports n from 1 to 20".into());
            }
            Ok(Value::Mat(Matrix::identity(k as usize)))
        }
        "dim" => {
            need(1, 1)?;
            match &a[0] {
                Value::Mat(m) => Ok(Value::List(vec![m.r as f64, m.c as f64])),
                Value::List(l) => Ok(Value::real(l.len() as f64)),
                _ => Err("dim() needs a matrix or a list".into()),
            }
        }
        "dot" | "cross" => {
            need(2, 2)?;
            let (u, v) = (list_of(&a[0])?, list_of(&a[1])?);
            if u.len() != v.len() {
                return Err("Vectors must have the same length".into());
            }
            if name == "dot" {
                Ok(Value::real(u.iter().zip(&v).map(|(p, q)| p * q).sum()))
            } else {
                if u.len() != 3 {
                    return Err("cross() needs two 3-element vectors".into());
                }
                Ok(Value::List(vec![
                    u[1] * v[2] - u[2] * v[1],
                    u[2] * v[0] - u[0] * v[2],
                    u[0] * v[1] - u[1] * v[0],
                ]))
            }
        }
        "Pol" => {
            need(2, 2)?;
            let (x, y) = (r(0)?, r(1)?);
            Ok(Value::List(vec![x.hypot(y), ang.from_rad(y.atan2(x))]))
        }
        "Rec" => {
            need(2, 2)?;
            let (rad, th) = (r(0)?, ang.to_rad(r(1)?));
            Ok(Value::List(vec![snap(rad * th.cos()), snap(rad * th.sin())]))
        }
        "and" | "or" | "xor" | "shl" | "shr" => {
            need(2, 2)?;
            let (x, y) = (int_of(&a[0])?, int_of(&a[1])?);
            let v = match name {
                "and" => x & y,
                "or" => x | y,
                "xor" => x ^ y,
                "shl" => {
                    if !(0..64).contains(&y) {
                        return Err("Shift must be between 0 and 63".into());
                    }
                    x << y
                }
                _ => {
                    if !(0..64).contains(&y) {
                        return Err("Shift must be between 0 and 63".into());
                    }
                    x >> y
                }
            };
            Ok(Value::real(v as f64))
        }
        "not" => need(1, 1).and_then(|_| Ok(Value::real(!int_of(&a[0])? as f64))),
        "normpdf" | "normcdf" | "invnorm" => {
            let base = if name == "normcdf" { 2 } else { 1 };
            if n != base && n != base + 2 {
                return Err(match name {
                    "normcdf" => "normcdf(lower, upper, μ, σ)".into(),
                    "normpdf" => "normpdf(x, μ, σ)".into(),
                    _ => "invnorm(p, μ, σ)".to_string(),
                });
            }
            let (mu, sigma) = if n == base + 2 { (r(base)?, r(base + 1)?) } else { (0.0, 1.0) };
            if sigma <= 0.0 {
                return Err("σ must be positive".into());
            }
            Ok(Value::real(match name {
                "normpdf" => stats::norm_pdf(r(0)?, mu, sigma),
                "normcdf" => stats::norm_cdf(r(1)?, mu, sigma) - stats::norm_cdf(r(0)?, mu, sigma),
                _ => {
                    let p = r(0)?;
                    if !(p > 0.0 && p < 1.0) {
                        return Err("The probability must be between 0 and 1".into());
                    }
                    mu + sigma * stats::inv_norm(p)
                }
            }))
        }
        "tpdf" | "tcdf" | "invt" | "chi2pdf" | "chi2cdf" | "invchi2" | "fpdf" | "fcdf" | "invf" => {
            use crate::hypothesis as h;
            let usage = match name {
                "tpdf" => "tpdf(x, df)",
                "tcdf" => "tcdf(lower, upper, df)",
                "invt" => "invt(p, df)",
                "chi2pdf" => "chi2pdf(x, df)",
                "chi2cdf" => "chi2cdf(lower, upper, df)",
                "invchi2" => "invchi2(p, df)",
                "fpdf" => "fpdf(x, df1, df2)",
                "fcdf" => "fcdf(lower, upper, df1, df2)",
                _ => "invf(p, df1, df2)",
            };
            let want = usage.matches(',').count() + 1;
            if n != want {
                return Err(format!("Use {}", usage));
            }
            let df_ok = |d: f64| -> Result<f64, String> {
                if d > 0.0 {
                    Ok(d)
                } else {
                    Err("Degrees of freedom must be positive".into())
                }
            };
            let prob = |p: f64| -> Result<f64, String> {
                if p > 0.0 && p < 1.0 {
                    Ok(p)
                } else {
                    Err("The probability must be between 0 and 1".into())
                }
            };
            Ok(Value::real(match name {
                "tpdf" => h::t_pdf(r(0)?, df_ok(r(1)?)?),
                "tcdf" => {
                    let v = df_ok(r(2)?)?;
                    h::t_cdf(r(1)?, v) - h::t_cdf(r(0)?, v)
                }
                "invt" => h::inv_t(prob(r(0)?)?, df_ok(r(1)?)?),
                "chi2pdf" => h::chi2_pdf(r(0)?, df_ok(r(1)?)?),
                "chi2cdf" => {
                    let k = df_ok(r(2)?)?;
                    h::chi2_cdf(r(1)?, k) - h::chi2_cdf(r(0)?, k)
                }
                "invchi2" => h::inv_chi2(prob(r(0)?)?, df_ok(r(1)?)?),
                "fpdf" => h::f_pdf(r(0)?, df_ok(r(1)?)?, df_ok(r(2)?)?),
                "fcdf" => {
                    let (d1, d2) = (df_ok(r(2)?)?, df_ok(r(3)?)?);
                    h::f_cdf(r(1)?, d1, d2) - h::f_cdf(r(0)?, d1, d2)
                }
                _ => h::inv_f(prob(r(0)?)?, df_ok(r(1)?)?, df_ok(r(2)?)?),
            }))
        }
        "binompdf" | "binomcdf" => {
            need(3, 3)?;
            let (k, trials, p) = (r(0)?, r(1)?, r(2)?);
            if trials < 0.0 || trials.fract() != 0.0 || !(0.0..=1.0).contains(&p) {
                return Err("Use (k, n, p) with whole n ≥ 0 and 0 ≤ p ≤ 1".into());
            }
            Ok(Value::real(if name == "binompdf" {
                stats::binom_pdf(k, trials, p)
            } else {
                stats::binom_cdf(k, trials, p)
            }))
        }
        "poissonpdf" | "poissoncdf" => {
            need(2, 2)?;
            let (k, lambda) = (r(0)?, r(1)?);
            if lambda <= 0.0 {
                return Err("λ must be positive".into());
            }
            Ok(Value::real(if name == "poissonpdf" {
                stats::poisson_pdf(k, lambda)
            } else {
                stats::poisson_cdf(k, lambda)
            }))
        }
        _ => Err(format!("Unknown function {}", name)),
    }
}

fn log_base(z: C, base: f64) -> Result<C, String> {
    if z.re == 0.0 && z.im == 0.0 {
        return Err("log of 0 is undefined".into());
    }
    if z.im == 0.0 && z.re > 0.0 {
        let v = if base == 10.0 {
            z.re.log10()
        } else if base == 2.0 {
            z.re.log2()
        } else if base == std::f64::consts::E {
            z.re.ln()
        } else {
            z.re.ln() / base.ln()
        };
        Ok(C::new(v, 0.0))
    } else {
        Ok(z.ln() / base.ln())
    }
}

pub fn eval(e: &Expr, env: &mut Env) -> R {
    match e {
        Expr::Num(v) => Ok(Value::real(*v)),
        Expr::Var(name) => match name.as_str() {
            "pi" => Ok(Value::real(PI)),
            "e" => Ok(Value::real(std::f64::consts::E)),
            "i" => Ok(Value::Num(C::new(0.0, 1.0))),
            "Ans" => Ok(env.ans.clone()),
            _ => env.vars.get(name).cloned().ok_or_else(|| format!("{} has no value yet", name)),
        },
        Expr::Neg(a) => match eval(a, env)? {
            Value::Num(z) => Ok(Value::Num(if z.im == 0.0 { C::new(-z.re, 0.0) } else { -z })),
            Value::Mat(m) => Ok(Value::Mat(m.scale(-1.0))),
            Value::List(l) => Ok(Value::List(l.iter().map(|v| -v).collect())),
        },
        Expr::Bin(op, a, b) => {
            // r∠θ: a complex number from its modulus and argument.
            if *op == '∠' {
                let (r, th) = (eval(a, env)?, eval(b, env)?);
                return match (r, th) {
                    (Value::Num(r), Value::Num(t)) if r.im == 0.0 && t.im == 0.0 => {
                        let rad = env.angle.to_rad(t.re);
                        Ok(Value::Num(clean(C::new(r.re * rad.cos(), r.re * rad.sin()))))
                    }
                    _ => Err("r∠θ needs two real numbers".into()),
                };
            }
            let x = eval(a, env)?;
            // 200 + 10% adds ten percent of 200; 200 − 10% takes it off.
            if let (true, Expr::Post('%', inner)) = (matches!(op, '+' | '-'), &**b) {
                if let Value::Num(xv) = &x {
                    if let Value::Num(pv) = eval(inner, env)? {
                        if xv.im == 0.0 && pv.im == 0.0 {
                            let k = 1.0 + if *op == '+' { pv.re } else { -pv.re } / 100.0;
                            return Ok(Value::real(xv.re * k));
                        }
                    }
                }
            }
            let y = eval(b, env)?;
            bin(*op, x, y)
        }
        Expr::Post(op, a) => {
            let v = eval(a, env)?;
            match op {
                '!' => map_real(&v, factorial),
                '%' => bin('/', v, Value::real(100.0)),
                '°' => {
                    let k = env.angle.from_rad(PI / 180.0);
                    bin('*', v, Value::real(k))
                }
                '²' => bin('^', v, Value::real(2.0)),
                '³' => bin('^', v, Value::real(3.0)),
                '⁻' => match v {
                    Value::Mat(m) => m.inverse().map(Value::Mat),
                    other => bin('/', Value::real(1.0), other),
                },
                _ => Err("Unknown operator".into()),
            }
        }
        Expr::Call(name, args) => {
            if let Some(r) = special(name, args, env) {
                return r;
            }
            let mut vals = Vec::with_capacity(args.len());
            for a in args {
                vals.push(eval(a, env)?);
            }
            call(name, &vals, env.angle)
        }
        Expr::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for it in items {
                out.push(real_of(&eval(it, env)?).map_err(|_| "Lists hold real numbers only".to_string())?);
            }
            Ok(Value::List(out))
        }
        Expr::Mat(rows) => {
            let mut out = Vec::with_capacity(rows.len());
            for row in rows {
                let mut r = Vec::with_capacity(row.len());
                for it in row {
                    r.push(real_of(&eval(it, env)?).map_err(|_| "Matrices hold real numbers only".to_string())?);
                }
                out.push(r);
            }
            Matrix::from_rows(out).map(Value::Mat)
        }
    }
}

fn check_finite(v: Value) -> R {
    let bad = match &v {
        Value::Num(z) => !z.re.is_finite() || !z.im.is_finite(),
        Value::Mat(m) => m.d.iter().any(|x| !x.is_finite()),
        Value::List(l) => l.iter().any(|x| !x.is_finite()),
    };
    if bad {
        Err("Result is undefined or too large".into())
    } else {
        Ok(v)
    }
}

/// Is `name` something a value can be stored into?
pub fn store_target(name: &str) -> bool {
    if name.starts_with("Mat") || name.starts_with("List") {
        return NAMED.contains(&name) && name != "MatAns";
    }
    let mut it = name.chars();
    match (it.next(), it.next()) {
        (Some(c), None) => c.is_alphabetic() && c != 'e' && c != 'i' && c != 'π',
        _ => false,
    }
}

/// Split "expr → X" into the expression and its store target.
pub fn split_store(src: &str) -> (&str, Option<&str>) {
    for arrow in ["→", "->"] {
        if let Some(pos) = src.rfind(arrow) {
            return (&src[..pos], Some(src[pos + arrow.len()..].trim()));
        }
    }
    (src, None)
}

/// "5 km to m", "98.6 °F in °C", "2.5 hours to minutes": a value with a unit converted to another.
/// Returns None when the line is not written that way.
pub fn unit_conversion(src: &str, env: &mut Env) -> Option<R> {
    let s = src.trim();
    let (lhs, to_unit) = [" to ", " in "].iter().find_map(|sep| s.rfind(sep).map(|i| (&s[..i], s[i + sep.len()..].trim())))?;
    let to = crate::units::find(to_unit)?;
    let lhs = lhs.trim();
    // The source unit is the last word; everything before it is the value.
    let (value_src, from_unit) = match lhs.rfind(|c: char| c.is_whitespace()) {
        Some(i) => (&lhs[..i], lhs[i..].trim()),
        None => {
            // "5km to m": peel the longest unit suffix off the number.
            let digits_end = lhs.char_indices().take_while(|(_, c)| c.is_ascii_digit() || *c == '.' || *c == '-').last().map(|(i, c)| i + c.len_utf8())?;
            (&lhs[..digits_end], lhs[digits_end..].trim())
        }
    };
    let from = crate::units::find(from_unit)?;
    if from.0 != to.0 {
        return Some(Err(format!("Cannot convert {} to {}", from.1.name, to.1.name)));
    }
    Some(match evaluate_pure(value_src, env) {
        Ok(Value::Num(z)) if z.im == 0.0 => Ok(Value::real(crate::units::convert(z.re, from.1, to.1))),
        Ok(_) => Err("A real number is needed before the unit".into()),
        Err(e) => Err(e),
    })
}

/// Evaluate without touching the environment (used for live previews).
pub fn evaluate_pure(src: &str, env: &mut Env) -> R {
    let (body, _) = split_store(src);
    if let Some(r) = unit_conversion(body, env) {
        return r;
    }
    check_finite(eval(&parse(body)?, env)?)
}

/// Evaluate a full input line, applying "→ X" storage and updating Ans.
pub fn evaluate(src: &str, env: &mut Env) -> R {
    let (body, target) = split_store(src);
    let v = match unit_conversion(body, env) {
        Some(r) => r?,
        None => check_finite(eval(&parse(body)?, env)?)?,
    };
    if let Some(t) = target {
        if !store_target(t) {
            return Err(format!("Cannot store into “{}”", t));
        }
        let ok = match (&v, t) {
            (Value::Mat(_), t) => t.starts_with("Mat"),
            (Value::List(_), t) => t.starts_with("List"),
            (Value::Num(_), t) => !t.starts_with("Mat") && !t.starts_with("List"),
        };
        if !ok {
            return Err(format!("That value does not fit in {}", t));
        }
        env.vars.insert(t.to_string(), v.clone());
    }
    if let Value::Mat(_) = &v {
        env.vars.insert("MatAns".into(), v.clone());
    }
    env.ans = v.clone();
    Ok(v)
}

/// Evaluate the text of an input field to a real number. Empty text gives `None`.
pub fn field_value(src: &str, env: &mut Env) -> Result<Option<f64>, String> {
    if src.trim().is_empty() {
        return Ok(None);
    }
    match evaluate_pure(src, env)? {
        Value::Num(z) if z.im == 0.0 => Ok(Some(z.re)),
        _ => Err("A real number is required".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(s: &str) -> f64 {
        let mut env = Env::default();
        match evaluate(s, &mut env) {
            Ok(Value::Num(z)) if z.im == 0.0 => z.re,
            other => panic!("{} -> {:?}", s, other),
        }
    }
    fn evr(s: &str) -> f64 {
        let mut env = Env::default();
        env.angle = Angle::Rad;
        evaluate(s, &mut env).unwrap().as_real().unwrap()
    }
    fn close(a: f64, b: f64) {
        assert!((a - b).abs() <= 1e-9 * b.abs().max(1.0), "{} vs {}", a, b);
    }
    fn err(s: &str) -> String {
        evaluate(s, &mut Env::default()).unwrap_err()
    }

    #[test]
    fn arithmetic() {
        close(ev("1+2*3"), 7.0);
        close(ev("(1+2)×3"), 9.0);
        close(ev("2^3^2"), 512.0);
        close(ev("-2^2"), -4.0);
        close(ev("2^-2"), 0.25);
        close(ev("10÷4"), 2.5);
        close(ev("7−10"), -3.0);
        close(ev("50%"), 0.5);
        close(ev("5!"), 120.0);
        close(ev("3²+4²"), 25.0);
        close(ev("2⁻¹"), 0.5);
        close(ev("1.5E3"), 1500.0);
        close(ev("2¹⁰"), 1024.0);
        close(ev("2⁻²"), 0.25);
        close(ev("-3²"), -9.0);
        close(ev("3⁴"), 81.0);
        close(ev("2E-3"), 0.002);
        close(ev("1e3"), 1000.0);
        close(ev("0xFF+0b101"), 260.0);
        close(ev("(2+3"), 5.0);
        close(ev("sin(30"), 0.5);
    }

    #[test]
    fn implicit_multiplication() {
        close(ev("2π"), 2.0 * PI);
        close(ev("2(3+4)"), 14.0);
        close(ev("(1+1)(2+2)"), 8.0);
        close(ev("1÷2π"), 1.0 / (2.0 * PI));
        close(ev("2sin(30)"), 1.0);
        close(ev("3√4"), 6.0);
        close(ev("2e"), 2.0 * std::f64::consts::E);
        let mut env = Env::default();
        evaluate("3→x", &mut env).unwrap();
        close(evaluate("2x^2", &mut env).unwrap().as_real().unwrap(), 18.0);
        close(evaluate("xsin(30)", &mut env).unwrap().as_real().unwrap(), 1.5);
    }

    #[test]
    fn functions() {
        close(ev("sin(30)"), 0.5);
        close(ev("cos(60)"), 0.5);
        close(ev("tan(45)"), 1.0);
        close(ev("asin(0.5)"), 30.0);
        assert_eq!(ev("sin(180)"), 0.0);
        assert_eq!(ev("cos(90)"), 0.0);
        assert_eq!(evr("sin(π)"), 0.0);
        close(evr("cos(π)"), -1.0);
        close(ev("ln(e)"), 1.0);
        close(ev("log(1000)"), 3.0);
        close(ev("log(2,8)"), 3.0);
        close(ev("√(16)"), 4.0);
        close(ev("cbrt(-27)"), -3.0);
        close(ev("root(5,-32)"), -2.0);
        close(ev("abs(-3)"), 3.0);
        close(ev("nCr(10,3)"), 120.0);
        close(ev("nPr(10,3)"), 720.0);
        close(ev("gcd(12,18)"), 6.0);
        close(ev("lcm(4,6)"), 12.0);
        close(ev("mod(-7,3)"), 2.0);
        close(ev("round(3.14159,2)"), 3.14);
        close(ev("0.5!"), 0.886226925452758);
        close(ev("and(12,10)"), 8.0);
        close(ev("90°") , 90.0);
        close(evr("sin(90°)"), 1.0);
        assert!(err("tan(90)").contains("undefined"));
        assert!(err("1/0").contains("zero"));
        assert!(err("sin 30").contains("parentheses"));
    }

    #[test]
    fn complex() {
        let mut env = Env::default();
        let z = match evaluate("(1+2i)(3-i)", &mut env).unwrap() {
            Value::Num(z) => z,
            _ => panic!(),
        };
        close(z.re, 5.0);
        close(z.im, 5.0);
        let z = match evaluate("√(-4)", &mut env).unwrap() {
            Value::Num(z) => z,
            _ => panic!(),
        };
        assert_eq!((z.re, z.im), (0.0, 2.0));
        env.angle = Angle::Rad;
        let z = match evaluate("e^(iπ)", &mut env).unwrap() {
            Value::Num(z) => z,
            _ => panic!(),
        };
        assert_eq!((z.re, z.im), (-1.0, 0.0));
        close(evaluate("abs(3+4i)", &mut env).unwrap().as_real().unwrap(), 5.0);
        close(evaluate("i^2", &mut env).unwrap().as_real().unwrap(), -1.0);
    }

    #[test]
    fn calculus() {
        close(ev("diff(x^3, 2)"), 12.0);
        close(ev("integ(x^2, 0, 3)"), 9.0);
        close(evr("integ(sin(x), 0, π)"), 2.0);
        close(evr("integ(sin(x)^2, 0, 2π)"), PI);
        close(ev("sum(k^2, k, 1, 10)"), 385.0);
        close(ev("prod(k, k, 1, 5)"), 120.0);
        close(ev("solve(x^2-2, 1)"), 2f64.sqrt());
        close(ev("solve(t^3-8, t, 1)"), 2.0);
        close(ev("sum(seq(n^2, n, 1, 4))"), 30.0);
    }

    #[test]
    fn variables_and_ans() {
        let mut env = Env::default();
        evaluate("5→A", &mut env).unwrap();
        close(evaluate("A^2", &mut env).unwrap().as_real().unwrap(), 25.0);
        close(evaluate("Ans+1", &mut env).unwrap().as_real().unwrap(), 26.0);
        assert!(evaluate("q+1", &mut env).unwrap_err().contains("no value"));
        assert!(evaluate("3→e", &mut env).is_err());
        // The bound variable of a special form does not leak.
        evaluate("integ(x,0,1)", &mut env).unwrap();
        assert!(!env.vars.contains_key("x"));
    }

    #[test]
    fn matrices_and_lists() {
        let mut env = Env::default();
        evaluate("[[1,2],[3,4]]→MatA", &mut env).unwrap();
        close(evaluate("det(MatA)", &mut env).unwrap().as_real().unwrap(), -2.0);
        let inv = evaluate("MatA⁻¹", &mut env).unwrap();
        match inv {
            Value::Mat(m) => {
                close(m.get(0, 0), -2.0);
                close(m.get(1, 1), -0.5);
            }
            _ => panic!(),
        }
        match evaluate("MatA×MatA⁻¹", &mut env).unwrap() {
            Value::Mat(m) => {
                close(m.get(0, 0), 1.0);
                assert!(m.get(0, 1).abs() < 1e-12);
            }
            _ => panic!(),
        }
        match evaluate("[1,2;3,4]^2", &mut env).unwrap() {
            Value::Mat(m) => assert_eq!(m.d, vec![7.0, 10.0, 15.0, 22.0]),
            _ => panic!(),
        }
        close(evaluate("rank([[1,2],[2,4]])", &mut env).unwrap().as_real().unwrap(), 1.0);
        close(evaluate("mean({1,2,3,4})", &mut env).unwrap().as_real().unwrap(), 2.5);
        close(evaluate("sum({1,2,3}×2)", &mut env).unwrap().as_real().unwrap(), 12.0);
        close(evaluate("dot({1,2,3},{4,5,6})", &mut env).unwrap().as_real().unwrap(), 32.0);
        assert!(evaluate("inv([[1,2],[2,4]])", &mut env).unwrap_err().contains("singular"));
    }

    #[test]
    fn distributions() {
        close(ev("normcdf(-1.96, 1.96)"), 0.950004209703559);
        close(ev("invnorm(0.975)"), 1.959963984540054);
        close(ev("binompdf(3, 10, 0.5)"), 0.1171875);
        close(ev("binomcdf(3, 10, 0.5)"), 0.171875);
        close(ev("invt(0.975, 10)"), 2.228138851986274);
        close(ev("tcdf(-2, 2, 10)"), 0.9266119652292598);
        close(ev("chi2cdf(0, 3.84, 1)"), 0.9499564787512949);
        close(ev("invf(0.95, 3, 20)"), 3.098391212140781);
        close(ev("poissonpdf(2, 3)"), 0.224041807655388);
    }

    #[test]
    fn numerics() {
        use crate::numeric::*;
        let r = find_roots(&mut |x| x * x - x - 6.0, -10.0, 10.0, 400);
        assert_eq!(r, vec![-2.0, 3.0]);
        let r = find_roots(&mut |x| (x - 1.0) * (x - 1.0), -10.0, 10.0, 400);
        assert_eq!(r.len(), 1);
        close(r[0], 1.0);
        assert!(find_roots(&mut |x| 1.0 / x, -10.0, 10.0, 401).is_empty());
        let ex = find_extrema(&mut |x| x * x - 2.0 * x - 3.0, -10.0, 10.0, 400);
        assert_eq!(ex.len(), 1);
        close(ex[0].x, 1.0);
        close(ex[0].y, -4.0);
        let p = poly_roots(&[1.0, -6.0, 11.0, -6.0]).unwrap();
        close(p[0].re, 1.0);
        close(p[1].re, 2.0);
        close(p[2].re, 3.0);
        let p = poly_roots(&[1.0, 0.0, 1.0]).unwrap();
        close(p[0].im.abs(), 1.0);
        assert_eq!(p[0].re, 0.0);
        let p = poly_roots(&[1.0, -2.0, 1.0]).unwrap();
        close(p[0].re, 1.0);
        close(p[1].re, 1.0);
        match linear_solve(&[vec![2.0, 1.0], vec![1.0, -1.0]], &[5.0, 1.0]) {
            LinSol::Unique(x) => {
                close(x[0], 2.0);
                close(x[1], 1.0);
            }
            _ => panic!(),
        }
        assert_eq!(linear_solve(&[vec![1.0, 1.0], vec![2.0, 2.0]], &[1.0, 2.0]), LinSol::Infinite);
        assert_eq!(linear_solve(&[vec![1.0, 1.0], vec![2.0, 2.0]], &[1.0, 3.0]), LinSol::NoSolution);
    }

    #[test]
    fn statistics() {
        use crate::stats::*;
        let s = one_var(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0], None).unwrap();
        close(s.mean, 5.0);
        close(s.pop_sd, 2.0);
        close(s.median, 4.5);
        close(s.q1, 4.0);
        close(s.q3, 6.0);
        assert_eq!(s.mode, vec![4.0]);
        let s = one_var(&[1.0, 2.0, 3.0], Some(&[1.0, 2.0, 1.0])).unwrap();
        close(s.n, 4.0);
        close(s.mean, 2.0);
        let xs = [1.0, 2.0, 3.0, 4.0, 5.0];
        let r = regress(Model::Linear, &xs, &[3.0, 5.0, 7.0, 9.0, 11.0]).unwrap();
        close(r.coef[0], 2.0);
        close(r.coef[1], 1.0);
        close(r.r.unwrap(), 1.0);
        let r = regress(Model::Quadratic, &xs, &[2.0, 5.0, 10.0, 17.0, 26.0]).unwrap();
        close(r.coef[0], 1.0);
        assert!(r.coef[1].abs() < 1e-8);
        close(r.coef[2], 1.0);
        let ys: Vec<f64> = xs.iter().map(|x| 3.0 * (0.5 * x).exp()).collect();
        let r = regress(Model::Exponential, &xs, &ys).unwrap();
        close(r.coef[0], 3.0);
        close(r.coef[1], 0.5);
        let ys: Vec<f64> = xs.iter().map(|x| 2.0 * x.powf(1.5)).collect();
        let r = regress(Model::Power, &xs, &ys).unwrap();
        close(r.coef[0], 2.0);
        close(r.coef[1], 1.5);
        close(r.predict(4.0), 16.0);
    }

    #[test]
    fn percent_polar_units() {
        assert!((ev("200 + 10%") - 220.0).abs() < 1e-9);
        assert!((ev("200 - 10%") - 180.0).abs() < 1e-9);
        assert!((ev("50%") - 0.5).abs() < 1e-9);
        assert!((ev("200 * 10%") - 20.0).abs() < 1e-9);
        let mut env = Env::default();
        match evaluate("2∠60", &mut env) {
            Ok(Value::Num(z)) => assert!((z.re - 1.0).abs() < 1e-9 && (z.im - 3f64.sqrt()).abs() < 1e-9),
            o => panic!("{:?}", o),
        }
        assert!((ev("5 km to m") - 5000.0).abs() < 1e-9);
        assert!((ev("5km to m") - 5000.0).abs() < 1e-9);
        assert!((ev("2.5 hours in minutes") - 150.0).abs() < 1e-9);
        assert!((ev("100 °C to °F") - 212.0).abs() < 1e-9);
        assert!((ev("3*2 km to m") - 6000.0).abs() < 1e-9);
        assert!(evaluate("5 km to kg", &mut env).is_err());
    }
}
