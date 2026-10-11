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
mod ui_prog;
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
    Prog,
}

const MODES: [(Mode, &str, Key); 8] = [
    (Mode::Calc, "Calculate", Key::Num1),
    (Mode::Graph, "Graph", Key::Num2),
    (Mode::Table, "Table", Key::Num3),
    (Mode::Equation, "Equation", Key::Num4),
    (Mode::Matrix, "Matrix", Key::Num5),
    (Mode::Stats, "Statistics", Key::Num6),
    (Mode::Convert, "Convert", Key::Num7),
    (Mode::Prog, "Programmer", Key::Num8),
];

const COMPACT_SIZE: [f32; 2] = [340.0, 580.0];
const COMPACT_MIN: [f32; 2] = [300.0, 500.0];
const FULL_SIZE: [f32; 2] = [1040.0, 700.0];
const FULL_MIN: [f32; 2] = [960.0, 600.0];

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
    prog: ui_prog::ProgState,
    settings: Settings,
    /// The keep-on-top state last sent to the window.
    applied_on_top: Option<bool>,
}

/// User preferences, saved between launches.
#[derive(Clone, PartialEq)]
struct Settings {
    /// 0 = follow the system, 1 = light, 2 = dark.
    appearance: usize,
    /// 0 none, 1 international, 2 Indian, 3 space.
    grouping: usize,
    /// 0 = decimal point, 1 = decimal comma.
    decimal: usize,
    on_top: bool,
}

impl Settings {
    fn load(storage: Option<&dyn eframe::Storage>) -> Self {
        let get = |k: &str, max: usize| -> usize {
            storage
                .and_then(|s| s.get_string(k))
                .and_then(|v| v.parse::<usize>().ok())
                .filter(|v| *v <= max)
                .unwrap_or(0)
        };
        Settings { appearance: get("appearance", 2), grouping: get("grouping", 3), decimal: get("decimal", 1), on_top: get("on_top", 1) == 1 }
    }
}

impl App {
    fn new(settings: Settings, history: Vec<String>) -> Self {
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
            prog: Default::default(),
            settings,
            applied_on_top: None,
        };
        app.mat.push(&mut app.env);
        app.stats.push(&mut app.env);
        // Replaying the saved history also brings back Ans and stored variables.
        app.calc.restore(&history, &mut app.env);
        app.mat.pull(&mut app.env);
        app.stats.pull(&mut app.env);
        #[cfg(debug_assertions)]
        app.debug_setup();
        app
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui, pal: &Pal) {
        ui.set_min_width(300.0);
        ui.spacing_mut().item_spacing.y = 4.0;
        let head = |ui: &mut egui::Ui, t: &str| {
            ui.add_space(6.0);
            ui.label(egui::RichText::new(t).font(theme::medium(12.0)).color(pal.dim));
        };
        head(ui, "Appearance");
        widgets::segmented(ui, pal, &mut self.settings.appearance, &["System", "Light", "Dark"]);
        head(ui, "Digit grouping");
        widgets::segmented(ui, pal, &mut self.settings.grouping, &["None", "1,234,567", "12,34,567", "1 234 567"]);
        head(ui, "Decimal mark");
        widgets::segmented(ui, pal, &mut self.settings.decimal, &["Point  1.5", "Comma  1,5"]);
        ui.label(egui::RichText::new("Changes how results look. Type a point for decimals.").size(11.5).color(pal.faint));
        head(ui, "Window");
        let mut top = self.settings.on_top as usize;
        if widgets::segmented(ui, pal, &mut top, &["Normal", "Keep on top"]) {
            self.settings.on_top = top == 1;
        }
        head(ui, "History");
        ui.horizontal(|ui| {
            let n = self.calc.history.len();
            ui.label(egui::RichText::new(format!("{} saved line{}", n, if n == 1 { "" } else { "s" })).size(12.5).color(pal.dim));
            if n > 0 && widgets::link(ui, pal, "Clear history") {
                self.calc.clear_history();
            }
        });
        ui.add_space(4.0);
        ui.label(egui::RichText::new(format!("Numera {}", help::VERSION)).size(11.5).color(pal.faint));
    }

    /// Development aid: NUMERA_MODE=1..7 opens a mode, NUMERA_DEMO=1 fills in sample content.
    #[cfg(debug_assertions)]
    fn debug_setup(&mut self) {
        if let Some(n) = std::env::var("NUMERA_MODE").ok().and_then(|v| v.parse::<usize>().ok()) {
            if (1..=MODES.len()).contains(&n) {
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
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let st = &self.settings;
        storage.set_string("appearance", st.appearance.to_string());
        storage.set_string("grouping", st.grouping.to_string());
        storage.set_string("decimal", st.decimal.to_string());
        storage.set_string("on_top", (st.on_top as usize).to_string());
        let hist = self.calc.history_exprs();
        let keep = &hist[hist.len().saturating_sub(200)..];
        storage.set_string("history", keep.join("\n"));
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.panel_fill.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(debug_assertions)]
        match std::env::var("NUMERA_THEME").as_deref() {
            Ok("dark") => self.settings.appearance = 2,
            Ok("light") => self.settings.appearance = 1,
            _ => {}
        }
        fmt::set_notation(self.settings.grouping as u8, self.settings.decimal == 1);
        if self.applied_on_top != Some(self.settings.on_top) {
            self.applied_on_top = Some(self.settings.on_top);
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(if self.settings.on_top {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            }));
        }
        ctx.set_theme(match self.settings.appearance {
            1 => egui::ThemePreference::Light,
            2 => egui::ThemePreference::Dark,
            _ => egui::ThemePreference::System,
        });
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
                        let help_clicked = widgets::glyph(ui, &pal, "?", 26.0).on_hover_text("Input guide: how to type powers, roots, functions and more").clicked();
                        if help_clicked {
                            self.show_help = !self.show_help;
                        }
                        let set_btn = widgets::settings_button(ui, &pal, 26.0).on_hover_text("Settings: appearance, number format, keep on top, history");
                        let pop_id = egui::Id::new("settings_popup");
                        if set_btn.clicked() {
                            ui.memory_mut(|m| m.toggle_popup(pop_id));
                        }
                        egui::popup::popup_below_widget(ui, pop_id, &set_btn, egui::popup::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                            self.settings_ui(ui, &pal);
                        });
                        let pin_tip = if self.settings.on_top { "Keeping Numera above other windows. Click to stop." } else { "Keep Numera above other windows" };
                        if widgets::pin_button(ui, &pal, 26.0, self.settings.on_top).on_hover_text(pin_tip).clicked() {
                            self.settings.on_top = !self.settings.on_top;
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
                Mode::Prog => {
                    if let Some(text) = ui_prog::ui(ui, &mut self.prog, &mut self.env, &pal) {
                        self.calc.input.push_str(&text);
                        self.calc.error = None;
                        self.mode = Mode::Calc;
                        self.calc.pending_cursor_end = true;
                    }
                }
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
    let options = eframe::NativeOptions { viewport, persist_window: false, ..Default::default() };
    eframe::run_native(
        "Numera",
        options,
        Box::new(|cc| {
            theme::install(&cc.egui_ctx);
            let settings = Settings::load(cc.storage);
            let history: Vec<String> = cc
                .storage
                .and_then(|s| s.get_string("history"))
                .map(|h| h.lines().filter(|l| !l.trim().is_empty()).map(String::from).collect())
                .unwrap_or_default();
            Ok(Box::new(App::new(settings, history)))
        }),
    )
}
