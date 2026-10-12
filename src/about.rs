//! The About window: what the app is, who made it, version and copyright.

use crate::help::{APP_NAME, BUILD_DATE, DEVELOPER, DEVELOPER_SITE, SOURCE_URL, VERSION};
use crate::theme::{self, Pal};
use eframe::egui::{self, Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, Stroke, TextureHandle};

/// The app icon as a texture, loaded the first time the window opens.
fn icon(ctx: &egui::Context, cache: &mut Option<TextureHandle>) -> Option<TextureHandle> {
    if cache.is_none() {
        let img = image::load_from_memory(include_bytes!("../packaging/icon-256.png")).ok()?.to_rgba8();
        let size = [img.width() as usize, img.height() as usize];
        let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
        *cache = Some(ctx.load_texture("about_icon", color, egui::TextureOptions::LINEAR));
    }
    cache.clone()
}

fn row(ui: &mut egui::Ui, pal: &Pal, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized([96.0, 20.0], egui::Label::new(RichText::new(label).font(theme::regular(13.0)).color(pal.dim)));
        add(ui);
    });
}

fn link(ui: &mut egui::Ui, pal: &Pal, text: &str, url: &str) {
    let r = ui
        .add(egui::Label::new(RichText::new(text).font(theme::regular(13.5)).color(pal.accent)).sense(egui::Sense::click()))
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(url);
    if r.clicked() {
        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
    }
}

pub fn window(ctx: &egui::Context, pal: &Pal, open: &mut bool, cache: &mut Option<TextureHandle>) {
    if !*open {
        return;
    }
    let tex = icon(ctx, cache);
    let year = &BUILD_DATE[..4];
    // Fit inside the small calculator window too.
    let screen_w = ctx.screen_rect().width();
    let narrow = screen_w < 480.0;
    let width = (screen_w - if narrow { 56.0 } else { 120.0 }).clamp(220.0, 380.0);
    let modal = egui::Modal::new(egui::Id::new("about_window"))
        .backdrop_color(Color32::from_black_alpha(if pal.dark { 150 } else { 90 }))
        .frame(
            Frame::new()
                .fill(pal.surface)
                .stroke(Stroke::new(1.0_f32, pal.line))
                .corner_radius(CornerRadius::same(16))
                .inner_margin(if narrow { Margin::symmetric(16, 18) } else { Margin::symmetric(28, 24) }),
        )
        .show(ctx, |ui| {
            // Fade and rise in.
            let t = crate::anim::appear(ctx, egui::Id::new("about_appear"), "open", 0.22);
            ui.set_opacity(t);
            ui.add_space((1.0 - t) * 12.0);
            ui.set_width(width);
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                if let Some(t) = &tex {
                    ui.add(egui::Image::new(t).fit_to_exact_size(egui::vec2(88.0, 88.0)));
                }
                ui.add_space(6.0);
                ui.label(RichText::new(APP_NAME).font(theme::light(26.0)).color(pal.text));
                ui.label(RichText::new(format!("Version {}  ·  Build {}", VERSION, BUILD_DATE)).font(theme::regular(13.0)).color(pal.dim));
                ui.add_space(12.0);
                ui.add(
                    egui::Label::new(
                        RichText::new(
                            "A clean, minimal scientific, financial and graphing calculator for study, engineering, business and everyday maths. \
                             Graphs, equations, matrices, statistics and hypothesis tests, loans and investments, unit conversion and a programmer mode, \
                             in one fast app that works completely offline and collects no data.",
                        )
                        .font(theme::regular(13.5))
                        .color(pal.text),
                    )
                    .wrap(),
                );
            });
            ui.add_space(14.0);
            ui.separator();
            ui.add_space(8.0);
            ui.spacing_mut().item_spacing.y = 6.0;
            row(ui, pal, "Created by", |ui| {
                ui.label(RichText::new(DEVELOPER).font(theme::medium(13.5)).color(pal.text));
            });
            row(ui, pal, "Website", |ui| link(ui, pal, DEVELOPER_SITE.trim_start_matches("https://"), DEVELOPER_SITE));
            row(ui, pal, "Source code", |ui| link(ui, pal, "GitHub", SOURCE_URL));
            row(ui, pal, "Version", |ui| {
                ui.label(RichText::new(format!("{} ({}, {})", VERSION, std::env::consts::OS, std::env::consts::ARCH)).font(theme::regular(13.5)));
            });
            row(ui, pal, "Built with", |ui| {
                ui.label(RichText::new("Rust and egui").font(theme::regular(13.5)));
            });
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                ui.label(RichText::new(format!("© {} {}. All rights reserved.", year, DEVELOPER)).font(theme::regular(12.5)).color(pal.dim));
                ui.label(RichText::new("Fonts: Inter (SIL Open Font License) and DejaVu Sans symbols.").font(theme::regular(11.5)).color(pal.faint));
                ui.add_space(12.0);
                if ui
                    .add(egui::Button::new(RichText::new("  Close  ").font(theme::medium(13.5))).corner_radius(CornerRadius::same(8)))
                    .clicked()
                {
                    *open = false;
                }
            });
        });
    if modal.should_close() {
        *open = false;
    }
}
