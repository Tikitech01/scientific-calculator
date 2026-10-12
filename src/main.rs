//! Scientific Calculator — a clean, minimal scientific and graphing calculator.
#![allow(dead_code)]
// No console window behind the app on Windows.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod engine;
mod fmt;
mod about;
mod help;
mod hypothesis;
mod matrix;
#[cfg(native_menu)]
mod menu;
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
mod ui_tests;
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

const SHORT_NAMES: [&str; 8] = ["Calc", "Graph", "Table", "Equation", "Matrix", "Stats", "Convert", "Prog"];

const COMPACT_SIZE: [f32; 2] = [340.0, 580.0];
const COMPACT_MIN: [f32; 2] = [300.0, 500.0];
const FULL_SIZE: [f32; 2] = [1100.0, 700.0];
/// Narrowest full window: wide enough that the mode tabs never run under the controls on the
/// right. macOS needs more room because the window buttons sit inside the top bar.
const FULL_MIN: [f32; 2] = if cfg!(target_os = "macos") { [1060.0, 600.0] } else { [990.0, 600.0] };

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
    show_about: bool,
    about_icon: Option<egui::TextureHandle>,
    /// Where the window should move when it changes size, so it stays on screen.
    pending_pos: Option<egui::Pos2>,
    /// The full calculator grew to the left (the small one sat on the right of the screen),
    /// so going back to the small one keeps the right edge in place.
    grew_left: bool,
    prog: ui_prog::ProgState,
    settings: Settings,
    /// The keep-on-top state last sent to the window.
    applied_on_top: Option<bool>,
    #[cfg(native_menu)]
    menu: Option<menu::AppMenu>,
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
            show_about: false,
            about_icon: None,
            pending_pos: None,
            grew_left: false,
            prog: Default::default(),
            settings,
            applied_on_top: None,
            #[cfg(native_menu)]
            menu: None,
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

    /// Whether the window sits in the right half of its screen.
    fn on_right_half(ctx: &egui::Context) -> bool {
        ctx.input(|i| {
            let vp = i.viewport();
            match (vp.outer_rect, vp.monitor_size) {
                (Some(r), Some(m)) if m.x > 0.0 => {
                    let x0 = (r.center().x / m.x).floor() * m.x;
                    r.center().x - x0 > m.x / 2.0
                }
                _ => false,
            }
        })
    }

    /// Top-left corner for the window at its new size: grow or shrink towards the middle of the
    /// screen (keep the right edge when the window is on the right), then keep it on screen.
    fn position_for(ctx: &egui::Context, new_inner: [f32; 2], keep_right: bool) -> Option<egui::Pos2> {
        ctx.input(|i| {
            let vp = i.viewport();
            let outer = vp.outer_rect?;
            let inner = vp.inner_rect.unwrap_or(outer);
            let mon = vp.monitor_size?;
            if mon.x <= 0.0 || mon.y <= 0.0 {
                return None;
            }
            let frame = outer.size() - inner.size();
            let w = new_inner[0] + frame.x;
            let h = new_inner[1] + frame.y;
            // Monitors side by side: find the one the window is on.
            let x0 = (outer.center().x / mon.x).floor() * mon.x;
            let y0 = (outer.center().y / mon.y).floor() * mon.y;
            let x = if keep_right { outer.right() - w } else { outer.left() };
            let x = x.min(x0 + mon.x - w).max(x0);
            let y = outer.top().min(y0 + mon.y - h).max(y0);
            Some(egui::pos2(x, y))
        })
    }

    fn set_compact(&mut self, ctx: &egui::Context, compact: bool) {
        if self.compact == compact {
            return;
        }
        let keep_right = if compact { self.grew_left } else { Self::on_right_half(ctx) };
        if !compact {
            self.grew_left = keep_right;
        }
        let size = if compact { COMPACT_SIZE } else { FULL_SIZE };
        self.pending_pos = Self::position_for(ctx, size, keep_right);
        self.compact = compact;
        self.size_applied = false;
        self.mode = Mode::Calc;
        self.calc.greek = false;
        self.calc.shift = false;
    }

    /// Carry out what was chosen in the menu bar, then update its check marks.
    #[cfg(native_menu)]
    fn menu_commands(&mut self, ctx: &egui::Context) {
        use menu::Cmd;
        let Some(m) = self.menu.as_mut() else { return };
        let cmds = m.poll();
        let key = |key: Key, shift: bool| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers { mac_cmd: true, command: true, shift, ..Default::default() },
        };
        for cmd in cmds {
            match cmd {
                Cmd::Settings => ctx.memory_mut(|m| m.open_popup(egui::Id::new("settings_popup"))),
                // The menu takes these keys before the window sees them, so hand them to the text fields.
                Cmd::Undo => ctx.input_mut(|i| i.events.push(key(Key::Z, false))),
                Cmd::Redo => ctx.input_mut(|i| i.events.push(key(Key::Z, true))),
                Cmd::Cut => ctx.input_mut(|i| i.events.push(egui::Event::Cut)),
                Cmd::Copy => ctx.input_mut(|i| i.events.push(egui::Event::Copy)),
                Cmd::Paste => {
                    let text = arboard::Clipboard::new().and_then(|mut c| c.get_text()).unwrap_or_default();
                    if !text.is_empty() {
                        ctx.input_mut(|i| i.events.push(egui::Event::Paste(text)));
                    }
                }
                Cmd::SelectAll => ctx.input_mut(|i| i.events.push(key(Key::A, false))),
                Cmd::CopyResult => self.calc.copy_last(ctx),
                Cmd::ClearHistory => self.calc.clear_history(),
                Cmd::ToggleFull => self.set_compact(ctx, !self.compact),
                Cmd::Mode(i) => {
                    self.set_compact(ctx, false);
                    self.mode = MODES[i.min(MODES.len() - 1)].0;
                }
                Cmd::Appearance(i) => self.settings.appearance = i,
                Cmd::Angle(i) => self.angle = i,
                Cmd::Format(i) => self.format = i,
                Cmd::KeepOnTop => self.settings.on_top = !self.settings.on_top,
                Cmd::Guide => self.show_help = !self.show_help,
                Cmd::Website => ctx.open_url(egui::OpenUrl::new_tab(help::SOURCE_URL)),
                Cmd::DeveloperSite => ctx.open_url(egui::OpenUrl::new_tab(help::DEVELOPER_SITE)),
                Cmd::About => self.show_about = true,
            }
        }
        let state = menu::MenuState {
            full: !self.compact,
            mode: MODES.iter().position(|m| m.0 == self.mode).unwrap_or(0),
            appearance: self.settings.appearance,
            angle: self.angle,
            format: self.format,
            on_top: self.settings.on_top,
        };
        if let Some(m) = self.menu.as_mut() {
            m.sync(&state);
        }
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
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{} {}  ·  build {}", help::APP_NAME, help::VERSION, help::BUILD_DATE))
                    .size(11.5)
                    .color(pal.faint),
            );
            if widgets::link(ui, pal, "About") {
                self.show_about = true;
                ui.memory_mut(|m| m.close_popup());
            }
        });
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
        self.show_about = std::env::var("NUMERA_ABOUT").is_ok();
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
        if let Some(k) = std::env::var("NUMERA_TEST").ok().and_then(|v| v.parse().ok()) {
            self.stats.tests.kind = k;
        }
        if let Some(k) = std::env::var("NUMERA_TSUB").ok().and_then(|v| v.parse().ok()) {
            self.stats.tests.t_kind = k;
            self.stats.tests.chi_kind = k;
            self.stats.tests.z_kind = k;
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
            self.stats.tests.groups = "23 25 21 27 24\n30 28 33 29 31\n18 22 20 19 21".into();
            self.stats.tests.table = "20 30 25\n15 35 40".into();
            self.stats.push(&mut self.env);
            self.mat.expr = "MatA⁻¹".into();
            self.mat.result = Some(engine::evaluate("MatA⁻¹", &mut self.env));
            self.stats.params[0] = ["100", "15", "85", "130", ""].map(String::from);
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
        #[cfg(native_menu)]
        self.menu_commands(ctx);
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
        // Keep tooltips inside the window, which matters most in the small calculator.
        let tip_w = (ctx.screen_rect().width() - 40.0).clamp(200.0, 420.0);
        if (ctx.style().spacing.tooltip_width - tip_w).abs() > 0.5 {
            ctx.style_mut(|s| s.spacing.tooltip_width = tip_w);
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
            if let Some(pos) = self.pending_pos.take() {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
            }
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
                    // The arrow points the way the window will grow or shrink: towards the
                    // middle of the screen, so the full calculator never opens off screen.
                    let left = if self.compact { Self::on_right_half(ctx) } else { !self.grew_left };
                    let arrow = if left { "‹" } else { "›" };
                    let tip = if self.compact {
                        "Show the full calculator: scientific keys, graphs, equations and more"
                    } else {
                        "Back to the simple calculator"
                    };
                    if widgets::glyph(ui, &pal, arrow, 28.0).on_hover_text(tip).clicked() {
                        self.set_compact(ctx, !self.compact);
                    }
                    if !self.compact {
                        ui.add_space(4.0);
                        // Use shorter labels when the window is too narrow for the full ones,
                        // so the tabs never run under the controls on the right.
                        let font = theme::medium(13.5);
                        let needed: f32 = MODES.iter().map(|(_, n, _)| widgets::text_width(ui, n, font.clone()) + 18.0).sum();
                        let short = ui.available_width() - 370.0 < needed;
                        for (i, (mode, name, _)) in MODES.iter().enumerate() {
                            let label = if short { SHORT_NAMES[i] } else { name };
                            if widgets::tab(ui, &pal, label, self.mode == *mode){
                                self.mode = *mode;
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
                        let pin_tip = if self.settings.on_top { "Staying above other windows. Click to stop." } else { "Keep the calculator above other windows" };
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

        about::window(ctx, &pal, &mut self.show_about, &mut self.about_icon);

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
                        self.calc.append(&text);
                        self.mode = Mode::Calc;
                    }
                }
                Mode::Convert => {
                    if let Some(text) = ui_convert::ui(ui, &mut self.conv, &mut self.env, f, &pal) {
                        // Focus is handed over on the next frame, once the input field exists.
                        self.calc.append(&text);
                        self.mode = Mode::Calc;
                    }
                }
                }
            });
    }
}

/// Name of the folder that holds saved settings and history.
const STORAGE_NAME: &str = "Scientific Calculator";

/// The app used to be called Numera: carry its saved settings and history over once.
fn migrate_settings() {
    let (Some(old), Some(new)) = (eframe::storage_dir("Numera"), eframe::storage_dir(STORAGE_NAME)) else {
        return;
    };
    let (old_file, new_file) = (old.join("app.ron"), new.join("app.ron"));
    if old_file.exists() && !new_file.exists() && std::fs::create_dir_all(&new).is_ok() {
        let _ = std::fs::copy(&old_file, &new_file);
    }
}

fn main() -> eframe::Result {
    let viewport = egui::ViewportBuilder::default()
        .with_title(help::APP_NAME)
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
    let viewport = viewport.with_app_id("scientific-calculator");
    // Development aid: NUMERA_X=900 opens the window at that x position.
    #[cfg(debug_assertions)]
    let viewport = match std::env::var("NUMERA_X").ok().and_then(|v| v.parse::<f32>().ok()) {
        Some(x) => viewport.with_position([x, 40.0]),
        None => viewport,
    };
    #[cfg(target_os = "macos")]
    let viewport = viewport.with_fullsize_content_view(true).with_titlebar_shown(false).with_title_shown(false);
    #[allow(unused_mut)]
    let mut options = eframe::NativeOptions { viewport, persist_window: false, ..Default::default() };
    // The app installs its own menu bar, so winit's default one is turned off.
    #[cfg(target_os = "macos")]
    {
        options.event_loop_builder = Some(Box::new(|b| {
            use winit::platform::macos::EventLoopBuilderExtMacOS;
            b.with_default_menu(false);
        }));
    }
    migrate_settings();
    eframe::run_native(
        STORAGE_NAME,
        options,
        Box::new(|cc| {
            theme::install(&cc.egui_ctx);
            let settings = Settings::load(cc.storage);
            let history: Vec<String> = cc
                .storage
                .and_then(|s| s.get_string("history"))
                .map(|h| h.lines().filter(|l| !l.trim().is_empty()).map(String::from).collect())
                .unwrap_or_default();
            #[allow(unused_mut)]
            let mut app = App::new(settings, history);
            #[cfg(native_menu)]
            {
                let names: Vec<&str> = MODES.iter().map(|m| m.1).collect();
                let formats: Vec<&str> = FORMATS.iter().map(|f| f.0).collect();
                let ctx = cc.egui_ctx.clone();
                app.menu = menu::AppMenu::install(&names, &formats, move || ctx.request_repaint());
            }
            Ok(Box::new(app))
        }),
    )
}
