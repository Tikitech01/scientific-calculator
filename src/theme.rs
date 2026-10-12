//! Colours, fonts and egui visuals.

use eframe::egui::{self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke};
use std::sync::Arc;

#[derive(Clone)]
pub struct Pal {
    pub dark: bool,
    pub bg: Color32,
    pub surface: Color32,
    pub key: Color32,
    pub key_hover: Color32,
    pub key_down: Color32,
    pub line: Color32,
    pub text: Color32,
    pub dim: Color32,
    pub faint: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub on_accent: Color32,
    pub danger: Color32,
    pub series: [Color32; 6],
}

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

impl Pal {
    pub fn light() -> Self {
        Pal {
            dark: false,
            bg: rgb(0xF7F6F3),
            surface: rgb(0xFFFFFF),
            key: rgb(0xECEAE5),
            key_hover: rgb(0xE3E0DA),
            key_down: rgb(0xD6D3CC),
            line: rgb(0xE2DFD9),
            text: rgb(0x1B1B1A),
            dim: rgb(0x75726C),
            faint: rgb(0xB4B0A8),
            accent: rgb(0xE2542B),
            accent_hover: rgb(0xD0471F),
            on_accent: rgb(0xFFFFFF),
            danger: rgb(0xC0392B),
            series: [rgb(0xE2542B), rgb(0x2F6FDE), rgb(0x1E9E6A), rgb(0x8A55D6), rgb(0xD4477F), rgb(0xC08A10)],
        }
    }

    pub fn dark() -> Self {
        Pal {
            dark: true,
            bg: rgb(0x121214),
            surface: rgb(0x1B1B1E),
            key: rgb(0x222226),
            key_hover: rgb(0x2B2B30),
            key_down: rgb(0x35353B),
            line: rgb(0x2B2B30),
            text: rgb(0xEDEDEB),
            dim: rgb(0x8E8E96),
            faint: rgb(0x55555C),
            accent: rgb(0xFF6A3D),
            accent_hover: rgb(0xFF7F58),
            on_accent: rgb(0x16110F),
            danger: rgb(0xFF6B6B),
            series: [rgb(0xFF6A3D), rgb(0x5B9BFF), rgb(0x3CC98A), rgb(0xAE83F5), rgb(0xF272A6), rgb(0xE3B341)],
        }
    }

    pub fn visuals(&self) -> egui::Visuals {
        let mut v = if self.dark { egui::Visuals::dark() } else { egui::Visuals::light() };
        v.panel_fill = self.bg;
        v.window_fill = self.surface;
        v.extreme_bg_color = self.surface;
        v.faint_bg_color = self.key;
        v.override_text_color = Some(self.text);
        v.hyperlink_color = self.accent;
        v.selection.bg_fill = mix(self.bg, self.accent, 0.28);
        v.selection.stroke = Stroke::new(1.0_f32, self.accent);
        v.text_cursor.stroke = Stroke::new(1.5_f32, self.accent);
        v.window_corner_radius = CornerRadius::same(10);
        v.menu_corner_radius = CornerRadius::same(10);
        v.window_stroke = Stroke::new(1.0_f32, self.line);
        v.popup_shadow = egui::Shadow {
            offset: [0, 6],
            blur: 18,
            spread: 0,
            color: Color32::from_black_alpha(if self.dark { 90 } else { 28 }),
        };
        let r = CornerRadius::same(7);
        let w = &mut v.widgets;
        w.noninteractive.bg_stroke = Stroke::new(1.0_f32, self.line);
        w.noninteractive.fg_stroke = Stroke::new(1.0_f32, self.text);
        w.noninteractive.corner_radius = r;
        for (state, fill) in [
            (&mut w.inactive, self.key),
            (&mut w.hovered, self.key_hover),
            (&mut w.active, self.key_down),
            (&mut w.open, self.key_hover),
        ] {
            state.bg_fill = fill;
            state.weak_bg_fill = fill;
            state.bg_stroke = Stroke::NONE;
            state.fg_stroke = Stroke::new(1.0_f32, self.text);
            state.corner_radius = r;
            state.expansion = 0.0;
        }
        v
    }
}

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

pub fn light(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("light".into()))
}

pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("medium".into()))
}

pub fn regular(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let defaults: Vec<String> = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    for (name, bytes) in [
        ("inter", &include_bytes!("../assets/Inter-Regular.ttf")[..]),
        ("inter-light", &include_bytes!("../assets/Inter-Light.ttf")[..]),
        ("inter-medium", &include_bytes!("../assets/Inter-Medium.ttf")[..]),
        // Maths symbols Inter lacks (∠, ⋚, arrows …), from DejaVu Sans.
        ("symbols", &include_bytes!("../assets/DejaVuSans-Symbols.ttf")[..]),
    ] {
        fonts.font_data.insert(name.to_string(), Arc::new(FontData::from_static(bytes)));
    }
    let chain = |first: &str| {
        let mut v = vec![first.to_string(), "symbols".to_string()];
        v.extend(defaults.iter().cloned());
        v
    };
    fonts.families.insert(FontFamily::Proportional, chain("inter"));
    fonts.families.insert(FontFamily::Monospace, chain("inter"));
    fonts.families.insert(FontFamily::Name("light".into()), chain("inter-light"));
    fonts.families.insert(FontFamily::Name("medium".into()), chain("inter-medium"));
    ctx.set_fonts(fonts);

    ctx.set_visuals_of(egui::Theme::Light, Pal::light().visuals());
    ctx.set_visuals_of(egui::Theme::Dark, Pal::dark().visuals());
    ctx.all_styles_mut(|s| {
        use egui::TextStyle::*;
        s.text_styles = [
            (Heading, regular(20.0)),
            (Body, regular(14.0)),
            (Button, regular(14.0)),
            (Small, regular(11.5)),
            (Monospace, regular(13.0)),
        ]
        .into();
        s.spacing.item_spacing = egui::vec2(8.0, 8.0);
        s.spacing.button_padding = egui::vec2(10.0, 5.0);
        s.spacing.scroll = egui::style::ScrollStyle::floating();
        s.interaction.selectable_labels = false;
    });
}
