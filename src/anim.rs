//! One place that decides how the interface animates: on or off, and how fast.
//!
//! Every hover fade, press, tab slide and panel transition asks here for its duration, so the
//! Settings switches apply everywhere at once. The graph never animates and is unaffected.

use eframe::egui::{self, Id};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(true);
/// 0 slow, 1 normal, 2 fast.
static SPEED: AtomicU8 = AtomicU8::new(1);

pub fn configure(ctx: &egui::Context, enabled: bool, speed: u8) {
    ENABLED.store(enabled, Ordering::Relaxed);
    SPEED.store(speed.min(2), Ordering::Relaxed);
    // egui's own transitions (side panel slide, scroll bars) follow the same setting.
    let t = time(0.16);
    if (ctx.style().animation_time - t).abs() > 1e-4 {
        ctx.style_mut(|s| s.animation_time = t);
    }
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// How long a transition with a normal duration of `base` seconds should take now.
pub fn time(base: f32) -> f32 {
    if !enabled() {
        return 0.0;
    }
    match SPEED.load(Ordering::Relaxed) {
        0 => base * 1.8,
        2 => base * 0.55,
        _ => base,
    }
}

/// 0 → 1 as `on` turns true, smoothly over the scaled duration (instant when animation is off).
pub fn toggle(ctx: &egui::Context, id: Id, on: bool, base: f32) -> f32 {
    ctx.animate_bool_with_time(id, on, time(base))
}

/// Smoothly follow a target value.
pub fn value(ctx: &egui::Context, id: Id, target: f32, base: f32) -> f32 {
    ctx.animate_value_with_time(id, target, time(base))
}

/// Ease-out curve for things that should start quickly and settle gently.
pub fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

/// Hover brightening for buttons.
pub fn hover(ctx: &egui::Context, id: Id, hovered: bool) -> f32 {
    toggle(ctx, id.with("hover"), hovered, 0.12)
}

/// Press depth for keys: rises fast on press, relaxes after release.
pub fn press(ctx: &egui::Context, id: Id, down: bool) -> f32 {
    let t = toggle(ctx, id.with("press"), down, if down { 0.05 } else { 0.18 });
    ease_out(t)
}

/// A short flash that starts when `trigger` is true for a frame (a key press, a new result)
/// and fades out on its own. Returns 1 at the start and 0 when finished.
pub fn flash(ctx: &egui::Context, id: Id, trigger: bool, base: f32) -> f32 {
    let key = id.with("flash_start");
    let now = ctx.input(|i| i.time);
    let dur = time(base) as f64;
    let start: Option<f64> = ctx.data(|d| d.get_temp(key));
    let start = if trigger {
        ctx.data_mut(|d| d.insert_temp(key, now));
        Some(now)
    } else {
        start
    };
    match start {
        Some(s) if dur > 0.0 && now - s < dur => {
            ctx.request_repaint();
            1.0 - ((now - s) / dur) as f32
        }
        _ => 0.0,
    }
}

/// Fade-and-rise for content that has just appeared (a mode switch, a popup). Returns the
/// progress 0 → 1; call `ui.set_opacity` and offset the content by `(1 - p) * rise` yourself.
pub fn appear(ctx: &egui::Context, id: Id, key: impl std::hash::Hash, base: f32) -> f32 {
    // Restart the transition whenever the key (e.g. the current mode) changes.
    let key_id = Id::new(key);
    let slot = id.with("appear_key");
    let now = ctx.input(|i| i.time);
    let prev: Option<Id> = ctx.data(|d| d.get_temp(slot));
    if prev != Some(key_id) {
        ctx.data_mut(|d| {
            d.insert_temp(slot, key_id);
            d.insert_temp(id.with("appear_t"), now);
        });
    }
    let start: f64 = ctx.data(|d| d.get_temp(id.with("appear_t"))).unwrap_or(0.0);
    let dur = time(base) as f64;
    if dur <= 0.0 {
        return 1.0;
    }
    let t = ((now - start) / dur) as f32;
    if t < 1.0 {
        ctx.request_repaint();
    }
    ease_out(t)
}
