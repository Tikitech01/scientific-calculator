//! The macOS menu bar: app, Edit, View, Window and Help menus.
//!
//! Menu shortcuts are caught by macOS before the window sees the key, so every command
//! (including Copy and Paste) comes back here as an event and is carried out by the app.

use muda::accelerator::Accelerator;
use muda::{AboutMetadata, CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use std::sync::mpsc::{channel, Receiver};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Cmd {
    Settings,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    CopyResult,
    ClearHistory,
    ToggleFull,
    Mode(usize),
    Appearance(usize),
    Angle(usize),
    Format(usize),
    KeepOnTop,
    Guide,
    Website,
}

/// What the check marks in the menus should show.
#[derive(Clone, PartialEq, Default)]
pub struct MenuState {
    pub full: bool,
    pub mode: usize,
    pub appearance: usize,
    pub angle: usize,
    pub format: usize,
    pub on_top: bool,
}

pub struct AppMenu {
    _menu: Menu,
    commands: Vec<(MenuId, Cmd)>,
    rx: Receiver<MenuEvent>,
    full: CheckMenuItem,
    modes: Vec<CheckMenuItem>,
    appearance: Vec<CheckMenuItem>,
    angle: Vec<CheckMenuItem>,
    format: Vec<CheckMenuItem>,
    on_top: CheckMenuItem,
    shown: Option<MenuState>,
}

fn acc(s: &str) -> Option<Accelerator> {
    s.parse().ok()
}

fn item(commands: &mut Vec<(MenuId, Cmd)>, text: &str, shortcut: &str, cmd: Cmd) -> MenuItem {
    let it = MenuItem::new(text, true, acc(shortcut));
    commands.push((it.id().clone(), cmd));
    it
}

fn check(commands: &mut Vec<(MenuId, Cmd)>, text: &str, shortcut: &str, cmd: Cmd) -> CheckMenuItem {
    let it = CheckMenuItem::new(text, true, false, acc(shortcut));
    commands.push((it.id().clone(), cmd));
    it
}

fn as_items(v: &[CheckMenuItem]) -> Vec<&dyn muda::IsMenuItem> {
    v.iter().map(|i| i as &dyn muda::IsMenuItem).collect()
}

impl AppMenu {
    /// Build the menu bar and install it. Call once the app is running, on the main thread.
    /// `wake` is called whenever a menu item is chosen so the app can redraw and handle it.
    pub fn install(
        mode_names: &[&str],
        format_names: &[&str],
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Option<AppMenu> {
        let (tx, rx) = channel();
        let tx = std::sync::Mutex::new(tx);
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            if let Ok(tx) = tx.lock() {
                let _ = tx.send(e);
            }
            wake();
        }));

        let mut commands: Vec<(MenuId, Cmd)> = Vec::new();
        let sep = PredefinedMenuItem::separator;

        // ---- App menu
        let credits = format!(
            "Developed by {dev}\nMumbai, India\n\n\
             Scientific, graphing, equation, matrix, statistics, conversion and programmer modes.\n\n\
             Built with Rust and egui.\n\
             Interface font: Inter (SIL Open Font License).\n\n\
             {site}",
            dev = crate::help::DEVELOPER,
            site = crate::help::WEBSITE.trim_start_matches("https://"),
        );
        let about = PredefinedMenuItem::about(
            Some("About Scientific Calculator"),
            Some(AboutMetadata {
                name: Some(crate::help::APP_NAME.into()),
                // macOS shows these as "Version 1.4.0 (Build 2026-10-11)".
                version: Some(crate::help::VERSION.into()),
                short_version: Some(format!("Build {}", crate::help::BUILD_DATE)),
                copyright: Some(format!("© {} {}", &crate::help::BUILD_DATE[..4], crate::help::DEVELOPER)),
                credits: Some(credits),
                authors: Some(vec![crate::help::DEVELOPER.into()]),
                comments: Some("A clean, minimal scientific and graphing calculator.".into()),
                website: Some(crate::help::WEBSITE.into()),
                ..Default::default()
            }),
        );
        let settings = item(&mut commands, "Settings…", "CmdOrCtrl+Comma", Cmd::Settings);
        let app_menu = Submenu::with_items(
            crate::help::APP_NAME,
            true,
            &[
                &about,
                &sep(),
                &settings,
                &sep(),
                &PredefinedMenuItem::services(None),
                &sep(),
                &PredefinedMenuItem::hide(Some("Hide Scientific Calculator")),
                &PredefinedMenuItem::hide_others(None),
                &PredefinedMenuItem::show_all(None),
                &sep(),
                &PredefinedMenuItem::quit(Some("Quit Scientific Calculator")),
            ],
        )
        .ok()?;

        // ---- Edit
        let undo = item(&mut commands, "Undo", "CmdOrCtrl+Z", Cmd::Undo);
        let redo = item(&mut commands, "Redo", "CmdOrCtrl+Shift+Z", Cmd::Redo);
        let cut = item(&mut commands, "Cut", "CmdOrCtrl+X", Cmd::Cut);
        let copy = item(&mut commands, "Copy", "CmdOrCtrl+C", Cmd::Copy);
        let paste = item(&mut commands, "Paste", "CmdOrCtrl+V", Cmd::Paste);
        let select_all = item(&mut commands, "Select All", "CmdOrCtrl+A", Cmd::SelectAll);
        let copy_result = item(&mut commands, "Copy Last Result", "CmdOrCtrl+Shift+C", Cmd::CopyResult);
        let clear_history = item(&mut commands, "Clear History", "", Cmd::ClearHistory);
        let edit_menu = Submenu::with_items(
            "Edit",
            true,
            &[&undo, &redo, &sep(), &cut, &copy, &paste, &select_all, &sep(), &copy_result, &clear_history],
        )
        .ok()?;

        // ---- View
        let full = check(&mut commands, "Full Calculator", "CmdOrCtrl+0", Cmd::ToggleFull);
        let modes: Vec<CheckMenuItem> = mode_names
            .iter()
            .enumerate()
            .map(|(i, name)| check(&mut commands, name, &format!("CmdOrCtrl+{}", i + 1), Cmd::Mode(i)))
            .collect();
        let appearance: Vec<CheckMenuItem> = ["System", "Light", "Dark"]
            .iter()
            .enumerate()
            .map(|(i, n)| check(&mut commands, n, "", Cmd::Appearance(i)))
            .collect();
        let angle: Vec<CheckMenuItem> = ["Degrees (DEG)", "Radians (RAD)", "Gradians (GRAD)"]
            .iter()
            .enumerate()
            .map(|(i, n)| check(&mut commands, n, "", Cmd::Angle(i)))
            .collect();
        let format: Vec<CheckMenuItem> =
            format_names.iter().enumerate().map(|(i, n)| check(&mut commands, n, "", Cmd::Format(i))).collect();
        let appearance_menu = Submenu::with_items("Appearance", true, &as_items(&appearance)).ok()?;
        let angle_menu = Submenu::with_items("Angle Unit", true, &as_items(&angle)).ok()?;
        let format_menu = Submenu::with_items("Number Format", true, &as_items(&format)).ok()?;
        let view_menu = Submenu::new("View", true);
        view_menu.append(&full).ok()?;
        view_menu.append(&sep()).ok()?;
        view_menu.append_items(&as_items(&modes)).ok()?;
        view_menu.append_items(&[&sep(), &appearance_menu, &angle_menu, &format_menu]).ok()?;

        // ---- Window
        let on_top = check(&mut commands, "Keep on Top", "", Cmd::KeepOnTop);
        let window_menu = Submenu::with_items(
            "Window",
            true,
            &[
                &PredefinedMenuItem::minimize(None),
                &PredefinedMenuItem::maximize(None),
                &PredefinedMenuItem::fullscreen(None),
                &sep(),
                &on_top,
                &sep(),
                &PredefinedMenuItem::bring_all_to_front(None),
            ],
        )
        .ok()?;

        // ---- Help
        let guide = item(&mut commands, "Input Guide", "CmdOrCtrl+Shift+Slash", Cmd::Guide);
        let website = item(&mut commands, "Scientific Calculator on GitHub", "", Cmd::Website);
        let help_menu = Submenu::with_items("Help", true, &[&guide, &website]).ok()?;

        let menu = Menu::with_items(&[&app_menu, &edit_menu, &view_menu, &window_menu, &help_menu]).ok()?;
        #[cfg(target_os = "macos")]
        {
            menu.init_for_nsapp();
            window_menu.set_as_windows_menu_for_nsapp();
            help_menu.set_as_help_menu_for_nsapp();
        }
        Some(AppMenu {
            _menu: menu,
            commands,
            rx,
            full,
            modes,
            appearance,
            angle,
            format,
            on_top,
            shown: None,
        })
    }

    /// Menu commands chosen since the last call.
    pub fn poll(&mut self) -> Vec<Cmd> {
        let mut out = Vec::new();
        while let Ok(e) = self.rx.try_recv() {
            if let Some((_, cmd)) = self.commands.iter().find(|(id, _)| *id == e.id) {
                out.push(*cmd);
            }
        }
        // macOS flips a check mark on its own when it is clicked, so set them all again.
        if !out.is_empty() {
            self.shown = None;
        }
        out
    }

    /// Bring the check marks in line with the app. Cheap when nothing changed.
    pub fn sync(&mut self, st: &MenuState) {
        if self.shown.as_ref() == Some(st) {
            return;
        }
        self.full.set_checked(st.full);
        for (i, it) in self.modes.iter().enumerate() {
            it.set_checked(st.full && i == st.mode);
        }
        for (i, it) in self.appearance.iter().enumerate() {
            it.set_checked(i == st.appearance);
        }
        for (i, it) in self.angle.iter().enumerate() {
            it.set_checked(i == st.angle);
        }
        for (i, it) in self.format.iter().enumerate() {
            it.set_checked(i == st.format);
        }
        self.on_top.set_checked(st.on_top);
        self.shown = Some(st.clone());
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn shortcuts_parse() {
        for s in [
            "CmdOrCtrl+Comma",
            "CmdOrCtrl+Z",
            "CmdOrCtrl+Shift+Z",
            "CmdOrCtrl+X",
            "CmdOrCtrl+C",
            "CmdOrCtrl+V",
            "CmdOrCtrl+A",
            "CmdOrCtrl+Shift+C",
            "CmdOrCtrl+0",
            "CmdOrCtrl+1",
            "CmdOrCtrl+8",
            "CmdOrCtrl+Shift+Slash",
        ] {
            assert!(super::acc(s).is_some(), "{}", s);
        }
    }
}
