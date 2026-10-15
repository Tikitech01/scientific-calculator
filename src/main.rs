//! Scientific Calculator — a clean, minimal scientific and graphing calculator.
#![allow(dead_code)]
// No console window behind the app on Windows.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod engine;
mod fmt;
mod about;
mod anim;
mod finance;
mod help;
mod hypothesis;
mod matrix;
#[cfg(native_menu)]
mod menu;
mod numeric;
mod plot;
mod stats;
mod theme;
mod sheet;
mod ui_calc;
mod ui_convert;
mod ui_equation;
mod ui_finance;
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

#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    Calc,
    Graph,
    Table,
    Equation,
    Matrix,
    Stats,
    Convert,
    Prog,
    Finance,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Mode::Calc => "Calculate",
            Mode::Graph => "Graph",
            Mode::Table => "Table",
            Mode::Equation => "Equation",
            Mode::Matrix => "Matrix",
            Mode::Stats => "Statistics",
            Mode::Convert => "Convert",
            Mode::Prog => "Programmer",
            Mode::Finance => "Finance",
        }
    }

    fn short(self) -> &'static str {
        match self {
            Mode::Calc => "Calc",
            Mode::Stats => "Stats",
            Mode::Prog => "Prog",
            Mode::Finance => "Finance",
            m => m.name(),
        }
    }
}

/// Every mode, in menu order.
const ALL_MODES: [Mode; 9] =
    [Mode::Calc, Mode::Graph, Mode::Table, Mode::Equation, Mode::Matrix, Mode::Stats, Mode::Finance, Mode::Convert, Mode::Prog];

const NUM_KEYS: [Key; 9] = [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7, Key::Num8, Key::Num9];

/// The kind of calculator: decides which modes are shown as tabs.
#[derive(Clone, Copy, PartialEq, Debug)]
enum CalcType {
    Scientific,
    Financial,
    Statistics,
    Programmer,
    All,
}

const TYPES: [CalcType; 5] = [CalcType::Scientific, CalcType::Financial, CalcType::Statistics, CalcType::Programmer, CalcType::All];

impl CalcType {
    fn name(self) -> &'static str {
        match self {
            CalcType::Scientific => "Scientific",
            CalcType::Financial => "Financial",
            CalcType::Statistics => "Statistics",
            CalcType::Programmer => "Programmer",
            CalcType::All => "All in one",
        }
    }

    fn modes(self) -> &'static [Mode] {
        match self {
            CalcType::Scientific => &[Mode::Calc, Mode::Graph, Mode::Table, Mode::Equation, Mode::Matrix, Mode::Convert],
            CalcType::Financial => &[Mode::Calc, Mode::Finance, Mode::Stats, Mode::Convert],
            CalcType::Statistics => &[Mode::Calc, Mode::Stats, Mode::Graph, Mode::Table, Mode::Finance],
            CalcType::Programmer => &[Mode::Calc, Mode::Prog, Mode::Convert],
            CalcType::All => &ALL_MODES,
        }
    }

    fn describe(self) -> String {
        match self {
            CalcType::All => "Every tool: scientific, financial, statistics and programmer".into(),
            _ => self.modes().iter().map(|m| m.name()).collect::<Vec<_>>().join(", "),
        }
    }
}

/// The File menu: sheets as plain text files, and the current view as CSV.
#[derive(Clone, Copy, PartialEq, Debug)]
enum FileCmd {
    New,
    Template(usize),
    Open,
    Save,
    SaveAs,
    ExportCsv,
}

const COMPACT_SIZE: [f32; 2] = [340.0, 580.0];
const COMPACT_MIN: [f32; 2] = [300.0, 500.0];
const FULL_SIZE: [f32; 2] = [1100.0, 700.0];
/// Narrowest full window: wide enough that the mode tabs never run under the controls on the
/// right. macOS needs more room because the window buttons sit inside the top bar.
const FULL_MIN: [f32; 2] = if cfg!(target_os = "macos") { [1120.0, 600.0] } else { [1050.0, 600.0] };
/// The all-in-one type has nine tabs, so its window must be wider still.
const ALL_MIN_WIDTH: f32 = if cfg!(target_os = "macos") { 1250.0 } else { 1180.0 };

const FORMATS: [(&str, Fmt); 7] = [
    ("Normal", Fmt::Norm),
    ("Fix 2", Fmt::Fix(2)),
    ("Fix 4", Fmt::Fix(4)),
    ("Fix 6", Fmt::Fix(6)),
    ("Sci 4", Fmt::Sci(4)),
    ("Sci 8", Fmt::Sci(8)),
    ("Eng", Fmt::Eng),
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
    /// The small calculator shows Settings as a page instead of a popup.
    show_settings: bool,
    /// The simple calculator: basic keys only, in a small window.
    compact: bool,
    size_applied: bool,
    /// The minimum window size matches the calculator type.
    min_applied: bool,
    show_about: bool,
    about_icon: Option<egui::TextureHandle>,
    /// Where the window should move when it changes size, so it stays on screen.
    pending_pos: Option<egui::Pos2>,
    /// The full calculator grew to the left (the small one sat on the right of the screen),
    /// so going back to the small one keeps the right edge in place.
    grew_left: bool,
    prog: ui_prog::ProgState,
    fin: ui_finance::FinState,
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
    /// Index into TYPES.
    calc_type: usize,
    /// Complex numbers: 0 = a + bi, 1 = r∠θ.
    polar: usize,
    /// Text size: 0 small, 1 normal, 2 large.
    text_size: usize,
    /// 0 = open as the simple calculator, 1 = open the full calculator.
    start_full: usize,
    /// Mode to return to, saved between launches (index into ALL_MODES).
    last_mode: usize,
    /// Interface animation: 0 off, 1 on.
    animate: usize,
    /// Animation speed: 0 slow, 1 normal, 2 fast.
    anim_speed: usize,
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
        Settings { appearance: get("appearance", 2), grouping: get("grouping", 3), decimal: get("decimal", 1), on_top: get("on_top", 1) == 1, calc_type: get("calc_type", TYPES.len() - 1), polar: get("polar", 1), text_size: get("text_size", 2).max(if storage.and_then(|s| s.get_string("text_size")).is_some() { 0 } else { 1 }), start_full: get("start_full", 1), last_mode: get("last_mode", ALL_MODES.len() - 1), animate: storage.and_then(|s| s.get_string("animate")).and_then(|v| v.parse().ok()).unwrap_or(1), anim_speed: storage.and_then(|s| s.get_string("anim_speed")).and_then(|v| v.parse().ok()).unwrap_or(1) }
    }
}

impl App {
    fn new(settings: Settings, history: Vec<String>, storage: Option<&dyn eframe::Storage>) -> Self {
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
            show_settings: false,
            compact: true,
            size_applied: true,
            min_applied: true,
            show_about: false,
            about_icon: None,
            pending_pos: None,
            grew_left: false,
            prog: Default::default(),
            fin: Default::default(),
            settings,
            applied_on_top: None,
            #[cfg(native_menu)]
            menu: None,
        };
        if app.settings.start_full == 1 {
            app.compact = false;
            app.size_applied = false;
        }
        let last = ALL_MODES[app.settings.last_mode.min(ALL_MODES.len() - 1)];
        if app.modes().contains(&last) {
            app.mode = last;
        }
        app.mat.push(&mut app.env);
        app.stats.push(&mut app.env);
        // Replaying the saved history also brings back Ans and stored variables.
        app.calc.restore(&history, &mut app.env);
        app.mat.pull(&mut app.env);
        app.stats.pull(&mut app.env);
        if let Some(st) = storage {
            app.calc.sheet_text = st.get_string("sheet").unwrap_or_default();
            app.calc.sheet_saved = st.get_string("sheet_saved").unwrap_or_default();
            app.calc.sheet_path = st.get_string("sheet_path").filter(|p| !p.is_empty()).map(std::path::PathBuf::from);
            app.calc.sheet = st.get_string("sheet_view").as_deref() == Some("1");
        }
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

    fn calc_type(&self) -> CalcType {
        TYPES[self.settings.calc_type.min(TYPES.len() - 1)]
    }

    /// The modes shown as tabs for the current calculator type.
    fn modes(&self) -> &'static [Mode] {
        self.calc_type().modes()
    }

    /// Switch calculator type; the current mode stays if the new type has it.
    fn set_type(&mut self, i: usize) {
        self.settings.calc_type = i.min(TYPES.len() - 1);
        if !self.modes().contains(&self.mode) {
            self.mode = Mode::Calc;
        }
        self.min_applied = false;
    }

    /// Smallest full window for the current type: wide enough for all its tabs.
    fn full_min(&self) -> [f32; 2] {
        if self.calc_type() == CalcType::All {
            [ALL_MIN_WIDTH, FULL_MIN[1]]
        } else {
            FULL_MIN
        }
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
        self.show_settings = false;
        self.size_applied = false;
        self.mode = Mode::Calc;
        self.calc.greek = false;
        self.calc.consts = false;
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
                Cmd::Settings => {
                    if self.compact {
                        self.show_settings = !self.show_settings;
                        self.show_help = false;
                    } else {
                        ctx.memory_mut(|m| m.open_popup(egui::Id::new("settings_popup")))
                    }
                }
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
                Cmd::ClearSheet => self.calc.clear_sheet(),
                Cmd::ToggleFull => self.set_compact(ctx, !self.compact),
                Cmd::Mode(i) => {
                    self.set_compact(ctx, false);
                    let mode = ALL_MODES[i.min(ALL_MODES.len() - 1)];
                    if !self.modes().contains(&mode) {
                        self.set_type(TYPES.len() - 1);
                    }
                    self.mode = mode;
                }
                Cmd::Type(i) => self.set_type(i),
                Cmd::Appearance(i) => self.settings.appearance = i,
                Cmd::Angle(i) => self.angle = i,
                Cmd::Format(i) => self.format = i,
                Cmd::KeepOnTop => self.settings.on_top = !self.settings.on_top,
                Cmd::Guide => self.show_help = !self.show_help,
                Cmd::Website => ctx.open_url(egui::OpenUrl::new_tab(help::SOURCE_URL)),
                Cmd::DeveloperSite => ctx.open_url(egui::OpenUrl::new_tab(help::DEVELOPER_SITE)),
                Cmd::Updates => ctx.open_url(egui::OpenUrl::new_tab(help::DOWNLOADS_URL)),
                Cmd::Feedback => ctx.open_url(egui::OpenUrl::new_tab(help::FEEDBACK_URL)),
                Cmd::About => self.show_about = true,
                Cmd::File(i) => {
                    let cmd = [FileCmd::New, FileCmd::Open, FileCmd::Save, FileCmd::SaveAs, FileCmd::ExportCsv][i.min(4)];
                    self.file_cmd(ctx, cmd);
                }
                Cmd::Template(i) => self.file_cmd(ctx, FileCmd::Template(i)),
            }
        }
        let state = menu::MenuState {
            full: !self.compact,
            mode: ALL_MODES.iter().position(|m| *m == self.mode).unwrap_or(0),
            calc_type: self.settings.calc_type,
            // Position of each mode among the tabs of the current type, for the ⌘1… shortcuts.
            shortcuts: ALL_MODES.map(|m| self.modes().iter().position(|x| *x == m)),
            appearance: self.settings.appearance,
            angle: self.angle,
            format: self.format,
            on_top: self.settings.on_top,
        };
        if let Some(m) = self.menu.as_mut() {
            m.sync(&state);
        }
    }

    /// Show the sheet view of Calculate (used after New, Open and a template).
    fn show_sheet(&mut self, ctx: &egui::Context) {
        self.set_compact(ctx, false);
        self.mode = Mode::Calc;
        self.calc.sheet = true;
        self.calc.pending_cursor_end = true;
    }

    /// What Export CSV writes in the current mode: a header and rows, or None when the mode
    /// has nothing tabular.
    fn csv_data(&mut self) -> Option<(Vec<String>, Vec<Vec<String>>)> {
        match self.mode {
            Mode::Calc if self.calc.sheet => Some(self.calc.sheet_csv()),
            Mode::Calc => Some(self.calc.history_csv()).filter(|d| !d.1.is_empty()),
            Mode::Table => ui_graph::table_rows(&mut self.graph, &mut self.env),
            Mode::Stats => {
                Some((vec!["X".into(), "Y".into(), "Freq".into()], self.stats.csv_rows())).filter(|d| !d.1.is_empty())
            }
            Mode::Matrix => Some((Vec::new(), self.mat.csv_rows())),
            _ => None,
        }
    }

    /// Suggested file name for Export CSV.
    fn csv_name(&self) -> String {
        match self.mode {
            Mode::Calc if self.calc.sheet => format!("{}.csv", self.calc.sheet_name()),
            Mode::Calc => "History.csv".into(),
            Mode::Matrix => format!("{}.csv", self.mat.name()),
            m => format!("{}.csv", m.name()),
        }
    }

    fn file_cmd(&mut self, ctx: &egui::Context, cmd: FileCmd) {
        let filter = |d: rfd::FileDialog| d.add_filter("Calculator sheet", &[sheet::EXT, "txt"]);
        match cmd {
            FileCmd::New => {
                self.calc.open_sheet(String::new(), None);
                self.show_sheet(ctx);
            }
            FileCmd::Template(i) => {
                let (_, text) = sheet::TEMPLATES[i.min(sheet::TEMPLATES.len() - 1)];
                self.calc.open_sheet(text.to_string(), None);
                self.show_sheet(ctx);
            }
            FileCmd::Open => {
                if let Some(path) = filter(rfd::FileDialog::new()).pick_file() {
                    match std::fs::read_to_string(&path) {
                        Ok(text) => self.calc.open_sheet(text, Some(path)),
                        Err(e) => self.calc.sheet_note = Some(format!("Could not open the file: {}", e)),
                    }
                    self.show_sheet(ctx);
                }
            }
            FileCmd::Save | FileCmd::SaveAs => {
                let path = match (&self.calc.sheet_path, cmd) {
                    (Some(p), FileCmd::Save) => Some(p.clone()),
                    _ => filter(rfd::FileDialog::new())
                        .set_file_name(format!("{}.{}", self.calc.sheet_name(), sheet::EXT))
                        .save_file(),
                };
                if let Some(path) = path {
                    match std::fs::write(&path, &self.calc.sheet_text) {
                        Ok(()) => {
                            self.calc.sheet_saved = self.calc.sheet_text.clone();
                            self.calc.sheet_path = Some(path);
                            self.calc.sheet_note = None;
                        }
                        Err(e) => self.calc.sheet_note = Some(format!("Could not save: {}", e)),
                    }
                }
                self.show_sheet(ctx);
            }
            FileCmd::ExportCsv => {
                let Some((head, rows)) = self.csv_data() else { return };
                let name = self.csv_name();
                if let Some(path) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).set_file_name(name).save_file() {
                    let head: Vec<&str> = head.iter().map(String::as_str).collect();
                    if let Err(e) = std::fs::write(&path, sheet::to_csv(&head, &rows)) {
                        self.calc.sheet_note = Some(format!("Could not export: {}", e));
                    }
                }
            }
        }
    }

    /// The File ▾ menu in the top bar.
    fn file_menu(&mut self, ui: &mut egui::Ui, pal: &Pal) {
        let mut cmd = None;
        let can_export = self.csv_data().is_some();
        let dirty = self.calc.sheet_dirty();
        let mac = cfg!(target_os = "macos");
        let key = |k: &str| if mac { format!("⌘{}", k) } else { format!("Ctrl+{}", k) };
        let r = ui.menu_button(egui::RichText::new("File ▾").size(12.5).color(pal.text), |ui| {
            ui.set_min_width(220.0);
            let item = |ui: &mut egui::Ui, label: &str, key: &str| {
                ui.horizontal(|ui| {
                    let r = ui.add(egui::Button::new(label).frame(false));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(egui::RichText::new(key).size(11.5).color(pal.faint));
                    });
                    r
                })
                .inner
                .clicked()
            };
            if item(ui, "New sheet", &key("N")) {
                cmd = Some(FileCmd::New);
            }
            ui.menu_button("New from template", |ui| {
                ui.set_min_width(200.0);
                for (i, (name, _)) in sheet::TEMPLATES.iter().enumerate() {
                    if ui.add(egui::Button::new(*name).frame(false)).clicked() {
                        cmd = Some(FileCmd::Template(i));
                        ui.close_menu();
                    }
                }
            })
            .response
            .on_hover_text("Worked sheets for engineering: change the values at the top and read the results");
            if item(ui, "Open…", &key("O")) {
                cmd = Some(FileCmd::Open);
            }
            ui.separator();
            if item(ui, if dirty { "Save •" } else { "Save" }, &key("S")) {
                cmd = Some(FileCmd::Save);
            }
            if item(ui, "Save as…", &if mac { "⇧⌘S".to_string() } else { "Ctrl+Shift+S".to_string() }) {
                cmd = Some(FileCmd::SaveAs);
            }
            ui.separator();
            ui.add_enabled_ui(can_export, |ui| {
                if item(ui, "Export CSV…", "") {
                    cmd = Some(FileCmd::ExportCsv);
                }
            })
            .response
            .on_hover_text(if can_export {
                "Save what is shown as a CSV file: the sheet or history, the table, the statistics data or the matrix"
            } else {
                "Nothing to export in this mode. Export works in Calculate, Table, Statistics and Matrix"
            });
            if cmd.is_some() {
                ui.close_menu();
            }
        });
        r.response.on_hover_text("Sheets: new, templates, open, save, and export as CSV");
        if let Some(cmd) = cmd {
            self.file_cmd(ui.ctx(), cmd);
        }
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui, pal: &Pal) {
        if !self.compact {
            ui.set_min_width(440.0);
            ui.set_max_width(440.0);
        } else {
            ui.set_max_width(ui.available_width().max(200.0));
        }
        let seg = |ui: &egui::Ui, l: &[&str]| widgets::segmented_width(ui, l);

        widgets::group(ui, pal, "General", |ui| {
            let names: Vec<&str> = TYPES.iter().map(|t| t.name()).collect();
            let mut t = self.settings.calc_type;
            let desc = self.calc_type().describe();
            widgets::setting_row(ui, pal, "Calculator type", &desc, seg(ui, &names), |ui| {
                if widgets::segmented(ui, pal, &mut t, &names) {
                    self.set_type(t);
                }
            });
            widgets::hairline(ui, pal);
            let l = ["Simple", "Full"];
            widgets::setting_row(ui, pal, "Open as", "What the app shows when it starts.", seg(ui, &l), |ui| {
                widgets::segmented(ui, pal, &mut self.settings.start_full, &l);
            });
            widgets::hairline(ui, pal);
            let l = ["Small", "Normal", "Large"];
            widgets::setting_row(ui, pal, "Text size", "", seg(ui, &l), |ui| {
                widgets::segmented(ui, pal, &mut self.settings.text_size, &l);
            });
        });

        widgets::group(ui, pal, "Appearance", |ui| {
            let l = ["System", "Light", "Dark"];
            widgets::setting_row(ui, pal, "Theme", "", seg(ui, &l), |ui| {
                widgets::segmented(ui, pal, &mut self.settings.appearance, &l);
            });
            widgets::hairline(ui, pal);
            let mut on = self.settings.animate == 1;
            widgets::setting_row(ui, pal, "Animation", "Fades, key presses and sliding tabs. Graphs never animate.", 42.0, |ui| {
                if widgets::toggle(ui, pal, &mut on) {
                    self.settings.animate = on as usize;
                }
            });
            if on {
                widgets::hairline(ui, pal);
                let l = ["Slow", "Normal", "Fast"];
                widgets::setting_row(ui, pal, "Animation speed", "", seg(ui, &l), |ui| {
                    widgets::segmented(ui, pal, &mut self.settings.anim_speed, &l);
                });
            }
        });

        widgets::group(ui, pal, "Numbers", |ui| {
            let l = ["None", "1,234,567", "12,34,567", "1 234 567"];
            widgets::setting_row(ui, pal, "Digit grouping", "", seg(ui, &l), |ui| {
                widgets::segmented(ui, pal, &mut self.settings.grouping, &l);
            });
            widgets::hairline(ui, pal);
            let l = ["Point  1.5", "Comma  1,5"];
            widgets::setting_row(ui, pal, "Decimal mark", "Changes how results look. Always type a point.", seg(ui, &l), |ui| {
                widgets::segmented(ui, pal, &mut self.settings.decimal, &l);
            });
            widgets::hairline(ui, pal);
            let l = ["a + bi", "r∠θ"];
            widgets::setting_row(ui, pal, "Complex numbers", "Type polar numbers as 5∠30 in either form.", seg(ui, &l), |ui| {
                widgets::segmented(ui, pal, &mut self.settings.polar, &l);
            });
        });

        widgets::group(ui, pal, "Window & data", |ui| {
            widgets::setting_row(ui, pal, "Keep on top", "Stay above other windows.", 42.0, |ui| {
                widgets::toggle(ui, pal, &mut self.settings.on_top);
            });
            widgets::hairline(ui, pal);
            let n = self.calc.history.len();
            let desc = format!("{} saved line{}, kept between launches.", n, if n == 1 { "" } else { "s" });
            widgets::setting_row(ui, pal, "History", &desc, 110.0, |ui| {
                ui.add_enabled_ui(n > 0, |ui| {
                    if widgets::link(ui, pal, "Clear history") {
                        self.calc.clear_history();
                    }
                });
            });
            widgets::hairline(ui, pal);
            let lines = self.calc.sheet_text.lines().filter(|l| !l.trim().is_empty()).count();
            let desc = match (&self.calc.sheet_path, self.calc.sheet_dirty()) {
                (Some(p), dirty) => format!("{}{} — kept between launches. Files are plain text (.calc).", p.display(), if dirty { " (unsaved changes)" } else { "" }),
                (None, _) => format!("{} line{}, not saved to a file yet. File ▾ Save as… keeps it as a plain text (.calc) file.", lines, if lines == 1 { "" } else { "s" }),
            };
            widgets::setting_row(ui, pal, "Sheet", &desc, 110.0, |ui| {
                ui.add_enabled_ui(lines > 0, |ui| {
                    if widgets::link(ui, pal, "Clear sheet") {
                        self.calc.clear_sheet();
                    }
                });
            });
        });

        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            if widgets::link(ui, pal, "About") {
                self.show_about = true;
                ui.memory_mut(|m| m.close_popup());
            }
            if widgets::link(ui, pal, "Send feedback") {
                ui.ctx().open_url(egui::OpenUrl::new_tab(help::FEEDBACK_URL));
            }
            if widgets::link(ui, pal, "Updates") {
                ui.ctx().open_url(egui::OpenUrl::new_tab(help::DOWNLOADS_URL));
            }
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!("{} {}  ·  build {}", help::APP_NAME, help::VERSION, help::BUILD_DATE))
                    .font(theme::regular(12.0))
                    .color(pal.faint),
            );
        });
        ui.add_space(6.0);
    }

    /// Development aid: SCICALC_MODE=1..7 opens a mode, SCICALC_DEMO=1 fills in sample content.
    #[cfg(debug_assertions)]
    fn debug_setup(&mut self) {
        if let Some(n) = std::env::var("SCICALC_TYPE").ok().and_then(|v| v.parse::<usize>().ok()) {
            self.set_type(n.saturating_sub(1));
        }
        if let Some(n) = std::env::var("SCICALC_MODE").ok().and_then(|v| v.parse::<usize>().ok()) {
            if (1..=ALL_MODES.len()).contains(&n) {
                self.mode = ALL_MODES[n - 1];
                if !self.modes().contains(&self.mode) {
                    self.settings.calc_type = TYPES.len() - 1;
                }
            }
        }
        self.show_help = std::env::var("SCICALC_HELP").is_ok();
        if std::env::var("SCICALC_POLAR").is_ok() {
            self.settings.polar = 1;
        }
        self.show_about = std::env::var("SCICALC_ABOUT").is_ok();
        if std::env::var("SCICALC_MODE").is_ok() || std::env::var("SCICALC_FULL").is_ok() {
            self.compact = false;
            self.size_applied = false;
        }
        self.calc.greek = std::env::var("SCICALC_GREEK").is_ok();
        // SCICALC_SHEET=1..5 opens that template in the sheet view.
        if let Some(n) = std::env::var("SCICALC_SHEET").ok().and_then(|v| v.parse::<usize>().ok()) {
            self.calc.open_sheet(sheet::TEMPLATES[n.clamp(1, sheet::TEMPLATES.len()) - 1].1.to_string(), None);
            self.compact = false;
            self.size_applied = false;
        }
        self.calc.consts = std::env::var("SCICALC_CONST").is_ok();
        if let Ok(sub) = std::env::var("SCICALC_SUB") {
            let s = sub.parse().unwrap_or(0);
            self.eq.sub = s;
            self.stats.sub = s;
            self.conv.sub = s;
            self.fin.sub = s;
        }
        if let Some(k) = std::env::var("SCICALC_TEST").ok().and_then(|v| v.parse().ok()) {
            self.stats.tests.kind = k;
        }
        if let Some(k) = std::env::var("SCICALC_TSUB").ok().and_then(|v| v.parse().ok()) {
            self.stats.tests.t_kind = k;
            self.stats.tests.chi_kind = k;
            self.stats.tests.z_kind = k;
            self.stats.tests.anova_kind = k;
            self.stats.tests.np_kind = k;
        }
        if std::env::var("SCICALC_DEMO").is_ok() {
            let eng = std::env::var("SCICALC_ENG").is_ok();
            let demo: &[&str] = if eng {
                &["10k ∥ 4.7k", "1/(2π·1kHz·100nF)", "kB·300/qe", "limit(sin(x)/x, x, 0)", "eig([[4,1],[2,3]])", "230÷(40+30i)", "(10∠30)×(5∠−45)"]
            } else {
                &["sin(30)+√(2)÷2", "√(2)÷2", "(1+2i)(3−i)", "integ(x²,0,3)", "[[1,2],[3,4]]→MatA", "2^64"]
            };
            for src in demo.iter().copied() {
                if let Ok(value) = engine::evaluate(src, &mut self.env) {
                    self.calc.history.push(ui_calc::Entry { expr: src.into(), value });
                }
            }
            self.calc.input = if eng { "√(3)×400×cos(36.87)".into() } else { "nCr(10,3)×5!".into() };
            self.mat.pull(&mut self.env);
            self.graph.fns = vec![
                ui_graph::FnEntry::new("x² − 2x − 3"),
                ui_graph::FnEntry::new("2x + 1"),
                ui_graph::FnEntry::new("3sin(60x)"),
            ];
            if std::env::var("SCICALC_ODE").is_ok() {
                self.graph.fns = vec![
                    ui_graph::FnEntry::new("y' = x − y, y(0) = 1"),
                    ui_graph::FnEntry::new("y'' = −4y − 0.4y', y(0) = 2, y'(0) = 0"),
                ];
                self.graph.slope_field = true;
            }
            if std::env::var("SCICALC_GRAPH2").is_ok() {
                self.graph.fns = vec![ui_graph::FnEntry::new("a·x² + b"), ui_graph::FnEntry::new("y < sin(k·x) + 2")];
                self.graph.tangent = true;
                self.graph.show_points = true;
                self.graph.fns[0].show_deriv = true;
            }
            self.graph.int_a = "-1".into();
            self.graph.int_b = "3".into();
            self.eq.deg = 1;
            self.eq.coef = ["1", "-6", "11", "-6", "", "", ""].map(String::from).to_vec();
            self.eq.sys[0] = ["2", "1", "", "", "", "", "5"].map(String::from).to_vec();
            self.eq.sys[1] = ["1", "-1", "", "", "", "", "1"].map(String::from).to_vec();
            self.eq.equation = "x³ − 2x = 5".into();
            self.stats.load_demo();
            self.fin.load_demo();
            self.prog.value = 0xBEEF;
            self.stats.tests.groups = "23 25 21 27 24\n30 28 33 29 31\n18 22 20 19 21".into();
            self.stats.tests.table = "20 30 25\n15 35 40".into();
            self.stats.tests.table2 = "4 5 6 | 6 7 8\n8 9 10 | 14 15 16".into();
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
        storage.set_string("calc_type", st.calc_type.to_string());
        storage.set_string("polar", st.polar.to_string());
        storage.set_string("text_size", st.text_size.to_string());
        storage.set_string("start_full", st.start_full.to_string());
        storage.set_string("animate", st.animate.to_string());
        storage.set_string("anim_speed", st.anim_speed.to_string());
        storage.set_string("last_mode", ALL_MODES.iter().position(|m| *m == self.mode).unwrap_or(0).to_string());
        let hist = self.calc.history_exprs();
        let keep = &hist[hist.len().saturating_sub(200)..];
        storage.set_string("history", keep.join("\n"));
        // The sheet comes back as it was, including unsaved changes.
        storage.set_string("sheet", self.calc.sheet_text.clone());
        storage.set_string("sheet_saved", self.calc.sheet_saved.clone());
        storage.set_string("sheet_path", self.calc.sheet_path.as_ref().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default());
        storage.set_string("sheet_view", (self.calc.sheet as usize).to_string());
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.panel_fill.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Development aid: SCICALC_PERF=1 prints how long each frame's UI takes.
        #[cfg(debug_assertions)]
        let perf_start = std::env::var("SCICALC_PERF").ok().map(|_| std::time::Instant::now());
        self.frame(ctx);
        #[cfg(debug_assertions)]
        if let Some(t) = perf_start {
            eprintln!("frame {:.2} ms", t.elapsed().as_secs_f64() * 1000.0);
            ctx.request_repaint();
        }
    }
}

impl App {
    fn frame(&mut self, ctx: &egui::Context) {
        #[cfg(native_menu)]
        self.menu_commands(ctx);
        #[cfg(debug_assertions)]
        match std::env::var("SCICALC_THEME").as_deref() {
            Ok("dark") => self.settings.appearance = 2,
            Ok("light") => self.settings.appearance = 1,
            _ => {}
        }
        fmt::set_notation(self.settings.grouping as u8, self.settings.decimal == 1);
        fmt::set_complex_form(self.settings.polar == 1, self.angle as u8);
        anim::configure(ctx, self.settings.animate == 1, self.settings.anim_speed as u8);
        let zoom = [0.9, 1.0, 1.15][self.settings.text_size.min(2)];
        if (ctx.zoom_factor() - zoom).abs() > 0.001 {
            ctx.set_zoom_factor(zoom);
        }
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

        // ⌘N ⌘O ⌘S ⇧⌘S: the File menu's shortcuts (on macOS the menu bar handles them).
        #[cfg(not(native_menu))]
        {
            let cmd = ctx.input(|i| {
                if !i.modifiers.command {
                    return None;
                }
                if i.key_pressed(Key::N) {
                    Some(FileCmd::New)
                } else if i.key_pressed(Key::O) {
                    Some(FileCmd::Open)
                } else if i.key_pressed(Key::S) {
                    Some(if i.modifiers.shift { FileCmd::SaveAs } else { FileCmd::Save })
                } else {
                    None
                }
            });
            if let Some(cmd) = cmd {
                self.file_cmd(ctx, cmd);
            }
        }
        // ⌘1 … ⌘9 switch between the tabs of the current type.
        if !self.compact {
            let modes = self.modes();
            ctx.input(|i| {
                if i.modifiers.command {
                    for (mode, key) in modes.iter().zip(NUM_KEYS) {
                        if i.key_pressed(key) {
                            self.mode = *mode;
                        }
                    }
                }
            });
        }
        if !self.size_applied {
            self.size_applied = true;
            self.min_applied = true;
            let full_min = self.full_min();
            let (size, min) = if self.compact { (COMPACT_SIZE, COMPACT_MIN) } else { ([FULL_SIZE[0].max(full_min[0]), FULL_SIZE[1]], full_min) };
            // Lower the minimum first so the window is free to shrink.
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(COMPACT_MIN.into()));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size.into()));
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(min.into()));
            if let Some(pos) = self.pending_pos.take() {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
            }
        } else if !self.min_applied && !self.compact {
            // A type with more tabs needs a wider window: raise the minimum and widen if needed.
            self.min_applied = true;
            let min = self.full_min();
            let cur = ctx.input(|i| i.viewport().inner_rect.map(|r| r.size()));
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(min.into()));
            if let Some(cur) = cur {
                if cur.x < min[0] - 0.5 {
                    // Keep the window on screen when it grows.
                    let new = [min[0], cur.y.max(min[1])];
                    let keep_right = Self::on_right_half(ctx);
                    if let Some(pos) = Self::position_for(ctx, new, keep_right) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(new.into()));
                }
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
                        ui.add_space(2.0);
                        // The calculator type: which tabs are shown.
                        let mut t = self.settings.calc_type;
                        if widgets::type_picker(ui, &pal, &mut t, &TYPES.map(|t| t.name()), &TYPES.map(|t| t.describe())) {
                            self.set_type(t);
                        }
                        ui.add_space(6.0);
                        // Use shorter labels when the window is too narrow for the full ones,
                        // so the tabs never run under the controls on the right.
                        let font = theme::medium(13.5);
                        let modes = self.modes();
                        let needed: f32 = modes.iter().map(|m| widgets::text_width(ui, m.name(), font.clone()) + 18.0).sum();
                        let short = ui.available_width() - 430.0 < needed;
                        for mode in modes {
                            let label = if short { mode.short() } else { mode.name() };
                            if widgets::tab(ui, &pal, label, self.mode == *mode) {
                                self.mode = *mode;
                            }
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        let help_clicked = widgets::glyph(ui, &pal, "?", 26.0).on_hover_text("Input guide: how to type powers, roots, functions, sheets and more").clicked();
                        if help_clicked {
                            self.show_help = !self.show_help;
                            self.show_settings = false;
                        }
                        let set_btn = widgets::settings_button(ui, &pal, 26.0).on_hover_text("Settings: appearance, number format, keep on top, history and sheet");
                        let pop_id = egui::Id::new("settings_popup");
                        if self.compact {
                            // Too narrow for a popup: Settings takes over the window instead.
                            if set_btn.clicked() {
                                self.show_settings = !self.show_settings;
                                self.show_help = false;
                            }
                        } else {
                            if set_btn.clicked() {
                                ui.memory_mut(|m| m.toggle_popup(pop_id));
                            }
                            egui::popup::popup_below_widget(ui, pop_id, &set_btn, egui::popup::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                                let t = anim::appear(ui.ctx(), pop_id.with("fade"), "open", 0.16);
                                ui.set_opacity(t);
                                ui.add_space((1.0 - t) * 6.0);
                                ui.set_max_height(ctx.screen_rect().height() - 90.0);
                                egui::ScrollArea::vertical().id_salt("settings_scroll").show(ui, |ui| {
                                    Frame::new().inner_margin(Margin { left: 10, right: 10, top: 8, bottom: 8 }).show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.add_space(4.0);
                                            ui.label(egui::RichText::new("Settings").font(theme::light(22.0)).color(pal.text));
                                        });
                                        ui.add_space(2.0);
                                        self.settings_ui(ui, &pal);
                                    });
                                });
                            });
                        }
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
                        self.file_menu(ui, &pal);
                    });
                });
            });

        let current = self.mode.name();
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
                if self.compact && self.show_settings {
                    let t = anim::appear(ctx, egui::Id::new("settings_page"), "open", 0.18);
                    ui.set_opacity(t);
                    ui.add_space((1.0 - t) * 8.0);
                    ui.add_space(4.0);
                    if widgets::page_header(ui, &pal, "Settings", "", true) {
                        self.show_settings = false;
                    }
                    egui::ScrollArea::vertical().id_salt("settings_page").auto_shrink([false, false]).show(ui, |ui| {
                        self.settings_ui(ui, &pal);
                        ui.add_space(12.0);
                    });
                    return;
                }
                // Switching modes: the new content fades and rises in (the graph is drawn at
                // full opacity so nothing on it ever animates).
                let key = (self.mode as u8, self.compact);
                let t = anim::appear(ctx, egui::Id::new("mode_switch"), key, 0.22);
                if !matches!(self.mode, Mode::Graph | Mode::Table) {
                    ui.set_opacity(t);
                    ui.add_space((1.0 - t) * 8.0);
                }
                match self.mode {
                Mode::Calc => {
                    if ui_calc::ui(ui, &mut self.calc, &mut self.env, f, &pal, self.compact) {
                        self.mat.pull(&mut self.env);
                        self.stats.pull(&mut self.env);
                    }
                }
                Mode::Graph => {
                    // The Statistics columns can be shown as points on the graph.
                    let (x, y) = (self.stats.list(0, &mut self.env), self.stats.list(1, &mut self.env));
                    self.graph.points = x.iter().zip(&y).map(|(a, b)| (*a, *b)).collect();
                    ui_graph::graph_ui(ui, &mut self.graph, &mut self.env, &pal)
                }
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
                Mode::Finance => ui_finance::ui(ui, &mut self.fin, &mut self.env, &pal),
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

/// Name of the settings folder from before the app was renamed.
const OLD_STORAGE_NAME: &str = "Numera";

/// Carry settings and history saved under the old name over once.
fn migrate_settings() {
    let (Some(old), Some(new)) = (eframe::storage_dir(OLD_STORAGE_NAME), eframe::storage_dir(STORAGE_NAME)) else {
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
    // Development aid: SCICALC_X=900 opens the window at that x position.
    #[cfg(debug_assertions)]
    let viewport = match std::env::var("SCICALC_X").ok().and_then(|v| v.parse::<f32>().ok()) {
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
            let mut app = App::new(settings, history, cc.storage);
            #[cfg(native_menu)]
            {
                let names: Vec<&str> = ALL_MODES.iter().map(|m| m.name()).collect();
                let types: Vec<(&str, String)> = TYPES.iter().map(|t| (t.name(), t.describe())).collect();
                let formats: Vec<&str> = FORMATS.iter().map(|f| f.0).collect();
                let templates: Vec<&str> = sheet::TEMPLATES.iter().map(|t| t.0).collect();
                let ctx = cc.egui_ctx.clone();
                app.menu = menu::AppMenu::install(&names, &types, &formats, &templates, move || ctx.request_repaint());
            }
            Ok(Box::new(app))
        }),
    )
}
