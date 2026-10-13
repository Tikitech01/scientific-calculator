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
Parametric: (cos(t), sin(t)).\n\
Differential equation: y' = x - y, y(0) = 1  or  y'' = -4y - 0.4y', y(0) = 2, y'(0) = 0.";
pub const NUMBER: &str = "A number or a calculation, e.g. 2.5, 1/3, sqrt(2), 2^10, pi.";
pub const EQUATION: &str = "An equation in x, e.g. x^3 - 2x = 5 or sin(x) = 0.5.\n\
Powers: type ^.  Without “=” the expression is solved for 0.";
pub const MATRIX_EXPR: &str = "Use MatA … MatF with + - * ^, e.g. MatA*MatB, MatA^2, 2MatA.\n\
Functions: det( ) inv( ) trn( ) rref( ) rank( ) trace( ) eig( ) eigvec( ) nullspace( ) lu( ) qr( ) expm( ) cond( ) identity(n).\n\
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
        "∥" => "Parallel: 10k ∥ 4.7k = 3.197k (also for impedances)",
        "CONST" => "Physical constants: c, h, kB, NA, ε₀, μ₀ … and ∥, dB, limit, taylor",
        _ => return None,
    })
}

const GENERAL: &[(&str, &str)] = &[
    ("Powers", "x^2, 2^10, x^-1. They turn into x², 2¹⁰, x⁻¹ as you type. Use brackets for anything longer: 2^(n+1), x^(1/3)."),
    ("Roots", "sqrt(2) or √(2).  cbrt(27).  root(4, 81) is the 4th root."),
    ("Multiply, divide", "* and /. The × may be left out: 2x, 3(4+1), 2sin(30), 2π."),
    ("Constants", "pi or π,  e,  i (imaginary unit).  Physical constants by name: c, g, G, h, hbar, qe, eV, me, mp, mn, amu, NA, kB, Rgas, eps0 (ε0), mu0 (μ0), Z0, kSB, atm — or 2nd + αβγ for the CONST keys. kB·300, NA·qe, 1/√(μ0·ε0). A variable you store with the same name wins."),
    ("SI prefixes", "Glued to a number: 4.7k, 220µ, 10n, 2.2M, 3p (T G M k m µ u n p f). A unit may follow and is ignored: 4.7kΩ, 220µF, 5mA, 3.3kV, 1MHz. To multiply by a variable instead, put a space or · first: 2 k, 2·m."),
    ("Engineering display", "Choose Eng in the Normal ▾ menu: results show with an SI prefix, 3.197k, 2.2m, 4.7µ."),
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
        ("Calculus", "diff(x^3, 2)   integ(x^2, 0, 3)   sum(k^2, k, 1, 10)   prod(k, k, 1, 5)   solve(x^2-2, 1).  They use the current angle unit."),
        ("More calculus", "Higher derivatives: diff(x^3, x, 2, 2) is f″(2); up to order 4. Partial: 3->x then diff(x²y³, y, 2). Improper: integ(e^(-x), 0, inf), integ(1/(1+x²), -inf, inf). Arc length arclen(x², x, 0, 1), volume of revolution about the x axis volume(√x, x, 0, 4), double integral integ2(x·y, x, 0, 1, y, 0, 2)."),
        ("Polynomials", "Coefficient lists, highest power first: {1,-6,11,-6} is x³−6x²+11x−6. polymul( ) polydiv( ) polyrem( ) polyeval(p, 2) polyroots( ) polyder( ) polyint( )."),
        ("Limits, series", "limit(sin(x)/x, x, 0)   limit((1+1/x)^x, x, inf).  taylor(e^x, x, 0, 4) gives the coefficients c₀ … c₄ of c₀ + c₁(x−a) + c₂(x−a)² + … Use RAD for the textbook series of sin and cos."),
        ("Circuits", "Parallel: 10k ∥ 4.7k (2nd + %, or type ||), works for complex impedances too.  dB(2) = 3.01 (power, 10·log₁₀), dBv(10) = 20 (amplitude, 20·log₁₀), invdB(3), invdBv(20)."),
        ("Lists", "{1,2,3}.  mean( ) median( ) stdev( ) sum( ) sort( )."),
        ("Number bases", "0xFF, 0b1010, 0o17.  and( ) or( ) xor( ) not( )."),
    ]),
    ("Graph", &[
        ("y = f(x)", "x^2 - 2x - 3,  sin(x),  1/x."),
        ("Polar", "Use θ (or type theta): 4cos(3theta)."),
        ("Parametric", "Two expressions in t inside brackets: (cos(t), sin(t))."),
        ("Differential equations", "y' = x − y, y(0) = 1  draws the solution through (0, 1). Second order: y'' = −4y − 0.4y', y(0) = 2, y'(0) = 0. Missing values default to y(0) = 1, y'(0) = 0. Solved numerically (Runge–Kutta); the Table tab lists its values. Show ▸ Slope field draws the slope y' = f(x, y) everywhere."),
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
        ("Eigenvalues", "eig(MatA) lists the eigenvalues (complex ones are shown as a message). eigvec(MatA) gives one unit eigenvector per column, in the same order; eigvec(MatA, 3) the eigenvector for λ = 3."),
        ("Decompositions", "lu(MatA) shows L stacked over U (P·A = L·U); lu(MatA, 1) is L, lu(MatA, 2) is U, lu(MatA, 0) is P. qr(MatA) shows Q over R; qr(MatA, 1) is Q, the Gram–Schmidt orthonormal basis of the columns. nullspace(MatA) has one basis vector per column. expm(MatA) is the matrix exponential, cond(MatA) the condition number."),
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
/// Where the newest downloads are listed.
pub const DOWNLOADS_URL: &str = "https://github.com/Tikitech01/scientific-calculator#download";
/// Bug reports and feature requests.
pub const FEEDBACK_URL: &str = "https://github.com/Tikitech01/scientific-calculator/issues/new";

/// One term and its explanation, as a row inside a group.
fn entry(ui: &mut egui::Ui, pal: &Pal, what: &str, how: &str, first: bool) {
    if !first {
        crate::widgets::hairline(ui, pal);
    }
    ui.add_space(10.0);
    ui.label(RichText::new(what).font(theme::medium(13.0)).color(pal.text));
    ui.add_space(3.0);
    ui.add(egui::Label::new(RichText::new(how).font(theme::regular(13.0)).color(pal.dim)).wrap().selectable(true));
    ui.add_space(10.0);
}

fn section(ui: &mut egui::Ui, pal: &Pal, title: &str, rows: &[(&str, &str)]) {
    crate::widgets::group(ui, pal, title, |ui| {
        for (i, (what, how)) in rows.iter().enumerate() {
            entry(ui, pal, what, how, i == 0);
        }
    });
}

/// Contents of the input guide. Returns true when the close button was pressed.
pub fn content(ui: &mut egui::Ui, pal: &Pal, current: &str) -> bool {
    ui.add_space(4.0);
    let close = crate::widgets::page_header(ui, pal, "Input guide", "How to type maths, and what each mode accepts.", true);
    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.set_max_width(ui.available_width());
        ui.add_space(2.0);
        // The mode in use first, then the general rules, then the other modes.
        for (title, rows) in SECTIONS.iter().filter(|s| s.0 == current) {
            section(ui, pal, &format!("{}  ·  this mode", title), rows);
        }
        section(ui, pal, "Typing maths", GENERAL);
        for (title, rows) in SECTIONS.iter().filter(|s| s.0 != current) {
            section(ui, pal, title, rows);
        }
        ui.add_space(22.0);
        ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = 3.0;
            ui.label(RichText::new("Hover any field or key for a short tip.").font(theme::regular(12.0)).color(pal.dim));
            ui.label(RichText::new(format!("{} {}  ·  build {}", APP_NAME, VERSION, BUILD_DATE)).font(theme::regular(11.5)).color(pal.faint));
            ui.label(RichText::new(format!("{}  ·  {}", DEVELOPER, DEVELOPER_SITE.trim_start_matches("https://"))).font(theme::regular(11.5)).color(pal.faint));
        });
        ui.add_space(20.0);
    });
    close
}

/// Full view: the guide docks to the right so the work stays visible.
pub fn side_panel(ctx: &egui::Context, open: &mut bool, pal: &Pal, current: &str) {
    let mut close = false;
    // The panel slides in from the right (and out again when closed).
    egui::SidePanel::right("input_guide")
        .exact_width(390.0)
        .resizable(false)
        .show_separator_line(true)
        .frame(egui::Frame::new().fill(pal.bg).inner_margin(egui::Margin { left: 22, right: 22, top: 14, bottom: 0 }))
        .show_animated(ctx, *open, |ui| {
            close = content(ui, pal, current);
        });
    if close {
        *open = false;
    }
}
