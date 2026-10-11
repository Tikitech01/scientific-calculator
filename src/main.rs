//! Numera — a clean, minimal scientific and graphing calculator.
#![allow(dead_code)]
// No console window behind the app on Windows.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod engine;
mod fmt;
mod help;
mod matrix;
mod numeric;
mod plot;
mod stats;
mod theme;
mod ui_calc;
mod ui_convert;
mod ui_equation;
mod ui_graph;
mod ui_matrix;
mod ui_stats;
mod units;
mod widgets;

use eframe::egui::{self, Align, ComboBox, Frame, Key, Layout, Margin};
use engine::{Angle, Env};
use fmt::Fmt;
use theme::Pal;

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Calc,
    Graph,
    Table,
    Equation,
    Matrix,
    Stats,
    Convert,
}

const MODES: [(Mode, &str, Key); 7] = [
    (Mode::Calc, "Calculate", Key::Num1),
    (Mode::Graph, "Graph", Key::Num2),
    (Mode::Table, "Table", Key::Num3),
    (Mode::Equation, "Equation", Key::Num4),
    (Mode::Matrix, "Matrix", Key::Num5),
    (Mode::Stats, "Statistics", Key::Num6),
    (Mode::Convert, "Convert", Key::Num7),
];

const COMPACT_SIZE: [f32; 2] = [340.0, 580.0];
const COMPACT_MIN: [f32; 2] = [300.0, 500.0];
const FULL_SIZE: [f32; 2] = [1000.0, 700.0];
const FULL_MIN: [f32; 2] = [860.0, 600.0];

const FORMATS: [(&str, Fmt); 6] = [
    ("Normal", Fmt::Norm),
    ("Fix 2", Fmt::Fix(2)),
    ("Fix 4", Fmt::Fix(4)),
    ("Fix 6", Fmt::Fix(6)),
    ("Sci 4", Fmt::Sci(4)),
    ("Sci 8", Fmt::Sci(8)),
];

struct App {
    mode: Mode,
    env: Env,
    angle: usize,
    format: usize,
    calc: ui_calc::CalcState,
    graph: ui_graph::GraphState,
    eq: ui_equation::EqState,
    mat: ui_matrix::MatState,
    stats: ui_stats::StatsState,
    conv: ui_convert::ConvState,
    show_help: bool,
    /// The simple calculator: basic keys only, in a small window.
    compact: bool,
    size_applied: bool,
}

impl App {
    fn new() -> Self {
        let mut app = App {
            mode: Mode::Calc,
            env: Env::default(),
            angle: 0,
            format: 0,
            calc: Default::default(),
            graph: Default::default(),
            eq: Default::default(),
            mat: Default::default(),
            stats: Default::default(),
            conv: Default::default(),
            show_help: false,
            compact: true,
            size_applied: true,
        };
        app.mat.push(&mut app.env);
        app.stats.push(&mut app.env);
        #[cfg(debug_assertions)]
        app.debug_setup();
        app
    }

    /// Development aid: NUMERA_MODE=1..7 opens a mode, NUMERA_DEMO=1 fills in sample content.
    #[cfg(debug_assertions)]
    fn debug_setup(&mut self) {
        if let Some(n) = std::env::var("NUMERA_MODE").ok().and_then(|v| v.parse::<usize>().ok()) {
            if (1..=7).contains(&n) {
                self.mode = MODES[n - 1].0;
            }
        }
        self.show_help = std::env::var("NUMERA_HELP").is_ok();
        if std::env::var("NUMERA_MODE").is_ok() || std::env::var("NUMERA_FULL").is_ok() {
            self.compact = false;
            self.size_applied = false;
        }
        self.calc.greek = std::env::var("NUMERA_GREEK").is_ok();
        if let Ok(sub) = std::env::var("NUMERA_SUB") {
            let s = sub.parse().unwrap_or(0);
            self.eq.sub = s;
            self.stats.sub = s;
            self.conv.sub = s;
        }
        if std::env::var("NUMERA_DEMO").is_ok() {
            for src in ["sin(30)+√(2)÷2", "√(2)÷2", "(1+2i)(3−i)", "integ(x²,0,3)", "[[1,2],[3,4]]→MatA", "2^64"] {
                if let Ok(value) = engine::evaluate(src, &mut self.env) {
                    self.calc.history.push(ui_calc::Entry { expr: src.into(), value });
                }
            }
            self.calc.input = "nCr(10,3)×5!".into();
            self.mat.pull(&mut self.env);
            self.graph.fns = vec![
                ui_graph::FnEntry::new("x² − 2x − 3"),
                ui_graph::FnEntry::new("2x + 1"),
                ui_graph::FnEntry::new("3sin(60x)"),
            ];
            self.graph.int_a = "-1".into();
            self.graph.int_b = "3".into();
            self.eq.deg = 1;
            self.eq.coef = ["1", "-6", "11", "-6", "", "", ""].map(String::from).to_vec();
            self.eq.sys[0] = ["2", "1", "", "", "", "", "5"].map(String::from).to_vec();
            self.eq.sys[1] = ["1", "-1", "", "", "", "", "1"].map(String::from).to_vec();
            self.eq.equation = "x³ − 2x = 5".into();
            self.stats.load_demo();
            self.stats.push(&mut self.env);
            self.mat.expr = "MatA⁻¹".into();
            self.mat.result = Some(engine::evaluate("MatA⁻¹", &mut self.env));
            self.stats.params = ["100", "15", "85", "130"].map(String::from);
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.panel_fill.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(debug_assertions)]
        match std::env::var("NUMERA_THEME").as_deref() {
            Ok("dark") => ctx.set_theme(egui::Theme::Dark),
            Ok("light") => ctx.set_theme(egui::Theme::Light),
            _ => {}
        }
        let pal = if ctx.style().visuals.dark_mode { Pal::dark() } else { Pal::light() };
        self.env.angle = [Angle::Deg, Angle::Rad, Angle::Grad][self.angle];
        let f = FORMATS[self.format].1;

        // ⌘1 … ⌘7 switch modes.
        if !self.compact {
            ctx.input(|i| {
                if i.modifiers.command {
                    for (mode, _, key) in MODES {
                        if i.key_pressed(key) {
                            self.mode = mode;
                        }
                    }
                }
            });
        }
        if !self.size_applied {
            self.size_applied = true;
            let (size, min) = if self.compact { (COMPACT_SIZE, COMPACT_MIN) } else { (FULL_SIZE, FULL_MIN) };
            // Lower the minimum first so the window is free to shrink.
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(COMPACT_MIN.into()));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size.into()));
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(min.into()));
        }

        // On macOS the window content runs under the title bar, so leave room for the traffic lights.
        let inset: i8 = if cfg!(target_os = "macos") { 84 } else { 16 };
        egui::TopBottomPanel::top("bar")
            .exact_height(48.0)
            .show_separator_line(false)
            .frame(Frame::new().fill(pal.bg).inner_margin(Margin { left: inset, right: 18, top: 9, bottom: 7 }))
            .show(ctx, |ui| {
                #[cfg(target_os = "macos")]
                {
                    let bar = ui.interact(ui.max_rect().expand(12.0), egui::Id::new("titlebar"), egui::Sense::click_and_drag());
                    if bar.drag_started() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }
                    if bar.double_clicked() {
                        let max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
                    }
                }
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let (arrow, tip) = if self.compact {
                        ("›", "Show the full calculator: scientific keys, graphs, equations and more")
                    } else {
                        ("‹", "Back to the simple calculator")
                    };
                    if widgets::glyph(ui, &pal, arrow, 28.0).on_hover_text(tip).clicked() {
                        self.compact = !self.compact;
                        self.size_applied = false;
                        self.mode = Mode::Calc;
                        self.calc.greek = false;
                        self.calc.shift = false;
                    }
                    if !self.compact {
                        ui.add_space(4.0);
                        for (mode, name, _) in MODES {
                            if widgets::tab(ui, &pal, name, self.mode == mode) {
                                self.mode = mode;
                            }
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        if widgets::glyph(ui, &pal, "?", 26.0).on_hover_text("Input guide: how to type powers, roots, functions and more").clicked() {
                            self.show_help = !self.show_help;
                        }
                        if self.compact {
                            return;
                        }
                        widgets::segmented(ui, &pal, &mut self.angle, &["DEG", "RAD", "GRAD"]);
                        ComboBox::from_id_salt("format")
                            .width(78.0)
                            .selected_text(egui::RichText::new(FORMATS[self.format].0).size(12.5))
                            .show_ui(ui, |ui| {
                                for (i, (name, _)) in FORMATS.iter().enumerate() {
                                    ui.selectable_value(&mut self.format, i, *name);
                                }
                            });
                    });
                });
            });

        let current = MODES.iter().find(|m| m.0 == self.mode).map(|m| m.1).unwrap_or("");
        if !self.compact {
            help::side_panel(ctx, &mut self.show_help, &pal, current);
        }

        egui::CentralPanel::default()
            .frame(Frame::new().fill(pal.bg).inner_margin(if self.compact {
                Margin { left: 14, right: 14, top: 6, bottom: 14 }
            } else {
                Margin { left: 22, right: 22, top: 10, bottom: 22 }
            }))
            .show(ctx, |ui| {
                // The small window shows the guide in place of the calculator.
                if self.compact && self.show_help {
                    if help::content(ui, &pal, current) {
                        self.show_help = false;
                    }
                    return;
                }
                match self.mode {
                Mode::Calc => {
                    if ui_calc::ui(ui, &mut self.calc, &mut self.env, f, &pal, self.compact) {
                        self.mat.pull(&mut self.env);
                        self.stats.pull(&mut self.env);
                    }
                }
                Mode::Graph => ui_graph::graph_ui(ui, &mut self.graph, &mut self.env, &pal),
                Mode::Table => ui_graph::table_ui(ui, &mut self.graph, &mut self.env, &pal),
                Mode::Equation => ui_equation::ui(ui, &mut self.eq, &mut self.env, f, &pal),
                Mode::Matrix => ui_matrix::ui(ui, &mut self.mat, &mut self.env, f, &pal),
                Mode::Stats => ui_stats::ui(ui, &mut self.stats, &mut self.env, f, &pal),
                Mode::Convert => {
                    if let Some(text) = ui_convert::ui(ui, &mut self.conv, &mut self.env, f, &pal) {
                        self.calc.input.push_str(&text);
                        self.calc.error = None;
                        self.mode = Mode::Calc;
                        // Focus is handed over on the next frame, once the input field exists.
                        self.calc.pending_cursor_end = true;
                    }
                }
                }
            });
    }
}

fn main() -> eframe::Result {
    let viewport = egui::ViewportBuilder::default()
        .with_title("Numera")
        .with_inner_size(COMPACT_SIZE)
        .with_min_inner_size(COMPACT_MIN)
        // An empty icon keeps the icon from the app bundle.
        .with_icon(std::sync::Arc::new(egui::IconData::default()));
    #[cfg(any(windows, target_os = "linux"))]
    let viewport = match eframe::icon_data::from_png_bytes(include_bytes!("../packaging/icon-256.png")) {
        Ok(icon) => viewport.with_icon(std::sync::Arc::new(icon)),
        Err(_) => viewport,
    };
    #[cfg(target_os = "linux")]
    let viewport = viewport.with_app_id("numera");
    #[cfg(target_os = "macos")]
    let viewport = viewport.with_fullsize_content_view(true).with_titlebar_shown(false).with_title_shown(false);
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native(
        "Numera",
        options,
        Box::new(|cc| {
            theme::install(&cc.egui_ctx);
            Ok(Box::new(App::new()))
        }),
    )
}
