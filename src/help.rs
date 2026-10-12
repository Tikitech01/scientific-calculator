//! Tooltips and the input guide.

use crate::theme::{self, Pal};
use eframe::egui::{self, RichText, ScrollArea};

pub const EXPR: &str = "Type a calculation and press return.\n\
Powers: x^2 (shown as x²)   Roots: sqrt(2), root(3,8)\n\
Multiply: * or just 2x, 3(4+1)   Divide: /\n\
Store: 5->A   Last answer: Ans   ↑ recalls earlier lines";
pub const FUNCTION: &str = "A function of x, for example x^2 - 2x - 3 or sin(x).\n\
Powers: type ^ (x^3).  Multiply: 2x or 2*x.\n\
Polar: use θ or write theta, e.g. 4cos(3theta).\n\
Parametric: (cos(t), sin(t)).";
pub const NUMBER: &str = "A number or a calculation, e.g. 2.5, 1/3, sqrt(2), 2^10, pi.";
pub const EQUATION: &str = "An equation in x, e.g. x^3 - 2x = 5 or sin(x) = 0.5.\n\
Powers: type ^.  Without “=” the expression is solved for 0.";
pub const MATRIX_EXPR: &str = "Use MatA … MatF with + - * ^, e.g. MatA*MatB, MatA^2, 2MatA.\n\
Functions: det( ) inv( ) trn( ) rref( ) rank( ) trace( ) identity(n).\n\
Inverse: MatA^-1.  Press return to calculate.";
pub const DATA: &str = "One value per row. Press tab to move to the next cell.\n\
Each cell accepts a number or a calculation.";
pub const FREQ: &str = "Optional. How many times the X value in this row occurs.";
pub const PROG_NOTE: &str = "Results wrap around at the word size, like a real register. In Calculate you can also write 0xFF, 0b1010 and use and( ) or( ) xor( ) not( ) shl( ) shr( ).";
pub const BASE: &str = "Whole numbers only. Hex digits are 0–9 and A–F.";

/// Tooltip for a keypad key, by its label.
pub fn key_tip(label: &str) -> Option<&'static str> {
    Some(match label {
        "2nd" => "Show the second function of each key (capitals on the Greek keys)",
        "αβγ" => "Greek letters. You can also type their names: alpha, beta, theta …",
        "x²" => "Square",
        "x³" => "Cube",
        "xʸ" => "Power. On the keyboard type ^, e.g. 2^10",
        "ʸ√x" => "Any root: root(3, 8) is the cube root of 8",
        "√" => "Square root. On the keyboard type sqrt(",
        "³√" => "Cube root",
        "x⁻¹" => "Reciprocal, 1 ÷ x",
        "mod" => "Remainder: mod(17, 5) = 2",
        "x!" => "Factorial",
        "Γ" => "Gamma function",
        "|x|" => "Absolute value",
        "arg" => "Angle of a complex number",
        "conj" => "Complex conjugate",
        "EXP" => "×10 to a power: 1.5E3 = 1500",
        "i" => "Imaginary unit, √(−1)",
        "e" => "Euler's number, 2.718…",
        "eˣ" => "e to a power",
        "10ˣ" => "10 to a power",
        "log" => "Base-10 logarithm. log(2, 8) uses base 2",
        "ln" => "Natural logarithm",
        "sin⁻¹" | "cos⁻¹" | "tan⁻¹" => "Inverse function; the answer is an angle in the current unit",
        "nCr" => "Combinations: nCr(10, 3)",
        "nPr" => "Permutations: nPr(10, 3)",
        "gcd" => "Greatest common divisor: gcd(12, 18)",
        "lcm" => "Least common multiple: lcm(4, 6)",
        "STO →" => "Store the result in a variable: 5 → A",
        "x" | "y" | "A" | "B" | "C" | "D" => "Variable. Give it a value with →, e.g. 5 → A",
        "," => "Separates the arguments of a function",
        "Ans" => "The last result",
        "round" => "Round: round(3.14159, 2) = 3.14",
        "a/b" => "Show results as fractions, multiples of π and roots when possible",
        "∫dx" => "Integral: integ(x^2, 0, 3) integrates x² from 0 to 3",
        "Σ" => "Sum: sum(k^2, k, 1, 10)",
        "d/dx" => "Derivative: diff(x^3, 2) is the slope of x³ at x = 2",
        "Π" => "Product: prod(k, k, 1, 5)",
        "solve" => "Solve for x: solve(x^2 - 2, 1) starts searching near 1",
        "Rand" => "Random number between 0 and 1",
        "AC" => "Clear the line. Press again to clear the history",
        "(−)" => "Negative sign",
        "%" => "Percent: 50% = 0.5",
        _ => return None,
    })
}

const GENERAL: &[(&str, &str)] = &[
    ("Powers", "x^2, 2^10, x^-1. They turn into x², 2¹⁰, x⁻¹ as you type. Use brackets for anything longer: 2^(n+1), x^(1/3)."),
    ("Roots", "sqrt(2) or √(2).  cbrt(27).  root(4, 81) is the 4th root."),
    ("Multiply, divide", "* and /. The × may be left out: 2x, 3(4+1), 2sin(30), 2π."),
    ("Constants", "pi or π,  e,  i (imaginary unit)."),
    ("Functions", "Always with brackets: sin(30), ln(2), log(100), log(2, 8), abs(-3)."),
    ("Scientific notation", "1.5E3 means 1.5 × 10³. Negative powers: 2E-3."),
    ("Greek letters", "Type the name: alpha, beta, theta, lambda, omega … becomes α, β, θ, λ, ω. Capital first letter gives the capital: Delta → Δ. Or use the αβγ key. They work as variables: 5->α."),
    ("Gamma", "gamma(5) is the gamma function. For the letter γ, type gamma followed by anything but a bracket, e.g. 2gamma+1."),
    ("Subscripts", "Not typed. Write log(2, 8) for log₂8, nCr(10, 3), MatA, List1."),
    ("Variables", "5->A stores 5 in A. Then use A anywhere. Ans is the last result."),
    ("Animation", "Keys sink when pressed, tabs and switches glide, panels and results fade in. Settings ▸ Animation turns this off or sets Slow / Normal / Fast. The graph itself never animates."),
    ("Text size, start-up", "Settings has Small / Normal / Large text, and whether the app opens as the simple or the full calculator. The last mode you used is remembered."),
    ("Calculator type", "The coloured button at the top left (or View ▸ Calculator Type, or Settings) picks Scientific, Financial, Statistics, Programmer or All in one. It only changes which tabs are shown; everything is still there under All in one. ⌘1, ⌘2 … follow the tabs you see."),
    ("Number format", "Digit grouping (1,234,567 or 12,34,567) and the decimal comma are in Settings. They change how results look; always type a point for decimals."),
];

const SECTIONS: &[(&str, &[(&str, &str)])] = &[
    ("Calculate", &[
        ("Evaluate", "Return or =.  ↑ ↓ recall earlier lines.  Esc clears."),
        ("Copy", "Click a result to copy it. Right-click it to copy it as shown, copy the whole line or insert it. ⌘⇧C (Ctrl+Shift+C) copies the latest result."),
        ("Reuse", "Click an earlier line to edit it. Ans is the last result."),
        ("History", "Kept between launches, along with stored variables. Press AC twice, or use Clear history in Settings, to clear it."),
        ("Complex numbers", "(1+2i)(3-i),  sqrt(-4),  abs(3+4i).  Polar form: 5∠30 (angle in the current unit). Settings can show results as r∠θ."),
        ("Percent", "200 + 10% = 220,  200 − 10% = 180,  50% = 0.5,  80 × 15% = 12."),
        ("Units", "5 km to m,  98.6 °F in °C,  2.5 hours to minutes,  3*2 kg in lb. Any unit from Convert works, by symbol or name."),
        ("Calculus", "diff(x^3, 2)   integ(x^2, 0, 3)   sum(k^2, k, 1, 10)   solve(x^2-2, 1)."),
        ("Lists", "{1,2,3}.  mean( ) median( ) stdev( ) sum( ) sort( )."),
        ("Number bases", "0xFF, 0b1010, 0o17.  and( ) or( ) xor( ) not( )."),
    ]),
    ("Graph", &[
        ("y = f(x)", "x^2 - 2x - 3,  sin(x),  1/x."),
        ("Polar", "Use θ (or type theta): 4cos(3theta)."),
        ("Parametric", "Two expressions in t inside brackets: (cos(t), sin(t))."),
        ("Inequalities", "y < x^2, y >= 2x + 1: the region is shaded; a strict inequality has a dashed edge."),
        ("Sliders", "Any other letter in a function (a·x² + b, sin(k·x)) gets a slider on the left. Press ▶ to animate it. The letters are ordinary variables, so Calculate sees them too."),
        ("Derivative, tangent", "Under Show: draw f′(x) of the selected function as a dashed line, and a tangent line with its slope wherever you hover."),
        ("Data points", "Values typed in Statistics (X and Y columns) can be drawn as points, to compare them with a model."),
        ("Move around", "Drag to pan. Scroll or pinch to zoom; hold Shift for x only, Option for y only. Double-click resets."),
        ("Analysis", "Click a function's field to analyse it. Hover the curve to trace it."),
        ("Intersections", "Every pair of visible curves is checked. Crossings are marked on the graph and listed on the left."),
        ("Export", "Export PNG saves the graph to your Downloads folder and copies it to the clipboard."),
        ("Angles", "sin, cos and tan follow the DEG / RAD / GRAD switch at the top right."),
    ]),
    ("Table", &[
        ("Range", "Start, end and step accept calculations, e.g. step 1/4 or pi/6."),
        ("Copy table", "Copies every row as tab-separated text, ready to paste into a spreadsheet. The Finance schedules have the same link."),
    ]),
    ("Equation", &[
        ("Coefficients", "Each box takes a number or a calculation: 1/3, sqrt(2). Empty means 0."),
        ("Solver", "Write the equation in x with one =, e.g. x^3 - 2x = 5. Widen the search range if a solution is missing."),
    ]),
    ("Matrix", &[
        ("Entries", "Each cell takes a number or a calculation. Empty means 0."),
        ("Expressions", "MatA*MatB,  MatA^2,  MatA^-1,  det(MatA),  trn(MatA),  2MatA + identity(2)."),
        ("By hand", "[[1,2],[3,4]] or [1,2;3,4]. In Calculate, [[1,2],[3,4]]->MatA stores it."),
    ]),
    ("Statistics", &[
        ("Data", "X and Y columns, one value per row. Freq is optional and repeats the X value."),
        ("Paste data", "Paste data… takes rows copied from a spreadsheet or a CSV file (tabs, commas or spaces between the columns). A header line is skipped."),
        ("Lists", "The columns are available in Calculate as List1, List2, List3."),
        ("Tests", "t-test (one sample, two samples, paired), z-test (mean, one or two proportions), F-test for variances, χ² goodness of fit and independence, one-way ANOVA with Tukey HSD, two-way ANOVA, Pearson and Spearman correlation. Each shows the statistic, p-value, critical value, confidence interval and whether to reject H₀ at your α."),
        ("Non-parametric", "Mann–Whitney U (two independent samples), Wilcoxon signed-rank (paired), Kruskal–Wallis (several groups) and the Jarque–Bera normality test. Use these when the data are not normal or are ranks."),
        ("Two-way ANOVA", "Type a grid: each line is a level of factor A, cells split by |, repeated values inside a cell separated by spaces. Every cell needs the same number of values."),
        ("Test data", "Most tests use the X and Y columns. The χ² independence table and ANOVA groups are typed one row or group per line, values separated by spaces or commas."),
        ("Distributions", "Normal, binomial, Poisson, Student t, χ² and F: probabilities between bounds, densities and inverse (critical) values."),
        ("In Calculate", "tcdf(lo, hi, df), invt(p, df), chi2cdf(lo, hi, df), invchi2(p, df), fcdf(lo, hi, d1, d2), invf(p, d1, d2), plus tpdf, chi2pdf, fpdf."),
    ]),
    ("Finance", &[
        ("Signs", "Money you pay out is negative, money you receive is positive. A loan: PV positive, PMT negative, FV 0."),
        ("Time value", "Fill in four of N, I/Y, PV, PMT, FV and press Solve beside the fifth. P/Y is payments per year, C/Y how often interest compounds."),
        ("Loan / EMI", "Amount, yearly rate and term give the monthly instalment and the full schedule. An extra monthly payment shows how much sooner the loan ends."),
        ("Interest", "Compound or simple interest, nominal ↔ effective rate conversion and CAGR between two values."),
        ("Cash flow", "CF0 is the investment at time 0 (negative); then one flow per line. NPV, IRR, MIRR, profitability index and payback."),
        ("Depreciation", "Straight line, declining balance, double declining and sum of years' digits, with the year-by-year book value."),
        ("Business", "Break-even, margin and markup, GST / VAT, percent change and discounts (30+10 stacks two discounts)."),
        ("Fields", "Every box takes a calculation: 2.5*100000, 15/12."),
    ]),
    ("Programmer", &[
        ("Value", "Type in the HEX, DEC, OCT or BIN row and the others follow. Spaces are ignored."),
        ("Bits", "Click any bit to flip it. Bit 0 is on the right."),
        ("Operations", "Enter B (12, 0xFF or 0b1010), then press AND, OR, XOR, <<, ROL, + and so on. The result replaces the value."),
        ("Word size", "8, 16, 32 or 64 bits. Signed shows the DEC row in two's complement."),
    ]),
    ("Convert", &[
        ("Units", "Type a value or a calculation, pick its unit, and read every other unit below."),
        ("Number base", "Type in any row. Whole numbers only."),
    ]),
];

pub const APP_NAME: &str = "Scientific Calculator";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BUILD_DATE: &str = env!("SCICALC_BUILD_DATE");
pub const DEVELOPER: &str = "Sopè Technology";
pub const DEVELOPER_SITE: &str = "https://sopetechnology.com";
pub const SOURCE_URL: &str = "https://github.com/Tikitech01/scientific-calculator";

fn heading(ui: &mut egui::Ui, pal: &Pal, title: &str) {
    ui.add_space(18.0);
    ui.label(RichText::new(title).font(theme::medium(15.5)).color(pal.text));
    ui.add_space(4.0);
    let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(r, 0.0, pal.line);
    ui.add_space(4.0);
}

fn entry(ui: &mut egui::Ui, pal: &Pal, what: &str, how: &str) {
    ui.add_space(7.0);
    ui.label(RichText::new(what).font(theme::medium(13.0)).color(pal.accent));
    ui.add_space(1.0);
    ui.add(egui::Label::new(RichText::new(how).font(theme::regular(14.0)).color(pal.text)).wrap().selectable(true));
}

/// Contents of the input guide. Returns true when the close button was pressed.
pub fn content(ui: &mut egui::Ui, pal: &Pal, current: &str) -> bool {
    let mut close = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Input guide").font(theme::light(24.0)).color(pal.text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if crate::widgets::glyph(ui, pal, "×", 26.0).on_hover_text("Close the guide").clicked() {
                close = true;
            }
        });
    });
    ui.add_space(2.0);
    ui.label(RichText::new("How to type maths, and what each mode accepts.").font(theme::regular(13.0)).color(pal.dim));
    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.set_max_width(ui.available_width());
        // The mode in use first, then the general rules, then the other modes.
        for (title, rows) in SECTIONS.iter().filter(|s| s.0 == current) {
            heading(ui, pal, &format!("{}  ·  this mode", title));
            for (what, how) in *rows {
                entry(ui, pal, what, how);
            }
        }
        heading(ui, pal, "Typing maths");
        for (what, how) in GENERAL {
            entry(ui, pal, what, how);
        }
        for (title, rows) in SECTIONS.iter().filter(|s| s.0 != current) {
            heading(ui, pal, title);
            for (what, how) in *rows {
                entry(ui, pal, what, how);
            }
        }
        ui.add_space(22.0);
        ui.label(RichText::new("Hover any field or key for a short tip.").font(theme::regular(12.5)).color(pal.dim));
        ui.label(RichText::new(format!("{} {}  ·  build {}", APP_NAME, VERSION, BUILD_DATE)).font(theme::medium(12.5)).color(pal.faint));
        ui.label(RichText::new(format!("Created by {}  ·  {}", DEVELOPER, DEVELOPER_SITE.trim_start_matches("https://"))).font(theme::regular(12.5)).color(pal.faint));
        ui.add_space(16.0);
    });
    close
}

/// Full view: the guide docks to the right so the work stays visible.
pub fn side_panel(ctx: &egui::Context, open: &mut bool, pal: &Pal, current: &str) {
    let mut close = false;
    // The panel slides in from the right (and out again when closed).
    egui::SidePanel::right("input_guide")
        .exact_width(360.0)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::new().fill(pal.surface).inner_margin(egui::Margin { left: 24, right: 20, top: 18, bottom: 0 }))
        .show_animated(ctx, *open, |ui| {
            close = content(ui, pal, current);
        });
    if close {
        *open = false;
    }
}
