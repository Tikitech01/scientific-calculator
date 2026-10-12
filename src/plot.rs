//! A small pannable, zoomable Cartesian plot surface.

use crate::fmt;
use crate::theme::{self, mix, Pal};
use eframe::egui::{self, pos2, Align2, Color32, Painter, Pos2, Rect, Shape, Stroke};

#[derive(Clone, Copy)]
pub struct View {
    pub cx: f64,
    pub cy: f64,
    /// World units per screen point.
    pub sx: f64,
    pub sy: f64,
}

impl Default for View {
    fn default() -> Self {
        View { cx: 0.0, cy: 0.0, sx: 0.03, sy: 0.03 }
    }
}

impl View {
    pub fn to_screen(&self, r: Rect, x: f64, y: f64) -> Pos2 {
        let px = r.center().x as f64 + (x - self.cx) / self.sx;
        let py = r.center().y as f64 - (y - self.cy) / self.sy;
        let lim = 1.0e5;
        pos2(px.clamp(-lim, lim) as f32, py.clamp(-lim, lim) as f32)
    }

    pub fn to_world(&self, r: Rect, p: Pos2) -> (f64, f64) {
        (
            self.cx + (p.x - r.center().x) as f64 * self.sx,
            self.cy - (p.y - r.center().y) as f64 * self.sy,
        )
    }

    pub fn x_range(&self, r: Rect) -> (f64, f64) {
        let h = r.width() as f64 * self.sx / 2.0;
        (self.cx - h, self.cx + h)
    }

    pub fn y_range(&self, r: Rect) -> (f64, f64) {
        let h = r.height() as f64 * self.sy / 2.0;
        (self.cy - h, self.cy + h)
    }

    pub fn fit(&mut self, r: Rect, x0: f64, x1: f64, y0: f64, y1: f64) {
        let (w, h) = ((x1 - x0).abs().max(1e-12), (y1 - y0).abs().max(1e-12));
        self.cx = 0.5 * (x0 + x1);
        self.cy = 0.5 * (y0 + y1);
        self.sx = w / (r.width() as f64).max(1.0);
        self.sy = h / (r.height() as f64).max(1.0);
    }

    /// Zoom by `fx`, `fy` (>1 zooms in) keeping the world point under `p` fixed.
    pub fn zoom(&mut self, r: Rect, p: Pos2, fx: f64, fy: f64) {
        let (wx, wy) = self.to_world(r, p);
        self.sx = (self.sx / fx).clamp(1e-10, 1e10);
        self.sy = (self.sy / fy).clamp(1e-10, 1e10);
        self.cx = wx - (p.x - r.center().x) as f64 * self.sx;
        self.cy = wy + (p.y - r.center().y) as f64 * self.sy;
    }
}

pub fn nice_step(raw: f64) -> f64 {
    let p = 10f64.powf(raw.log10().floor());
    let m = raw / p;
    let k = if m <= 1.0 {
        1.0
    } else if m <= 2.0 {
        2.0
    } else if m <= 5.0 {
        5.0
    } else {
        10.0
    };
    k * p
}

pub fn draw_grid(p: &Painter, r: Rect, v: &View, pal: &Pal) {
    let grid = Stroke::new(1.0_f32, mix(pal.bg, pal.line, if pal.dark { 0.75 } else { 0.8 }));
    let axis = Stroke::new(1.0_f32, pal.faint);
    let font = theme::regular(10.5);
    let (x0, x1) = v.x_range(r);
    let (y0, y1) = v.y_range(r);
    let step_x = nice_step(v.sx * 84.0);
    let step_y = nice_step(v.sy * 64.0);
    let origin = v.to_screen(r, 0.0, 0.0);
    // max/min rather than clamp: a very small plot must not panic.
    let label_y = (origin.y + 5.0).min(r.bottom() - 16.0).max(r.top() + 4.0);
    let label_x = (origin.x - 6.0).min(r.right() - 4.0).max(r.left() + 30.0);

    let mut k = (x0 / step_x).ceil();
    while k * step_x <= x1 {
        let x = k * step_x;
        let sx = v.to_screen(r, x, 0.0).x;
        if k != 0.0 {
            p.vline(sx, r.y_range(), grid);
        }
        if k != 0.0 && sx > r.left() + 22.0 && sx < r.right() - 22.0 {
            p.text(pos2(sx, label_y), Align2::CENTER_TOP, fmt::short(x, 6), font.clone(), pal.dim);
        }
        k += 1.0;
    }
    let mut k = (y0 / step_y).ceil();
    while k * step_y <= y1 {
        let y = k * step_y;
        let sy = v.to_screen(r, 0.0, y).y;
        if k != 0.0 {
            p.hline(r.x_range(), sy, grid);
            p.text(pos2(label_x, sy), Align2::RIGHT_CENTER, fmt::short(y, 6), font.clone(), pal.dim);
        }
        k += 1.0;
    }
    if origin.x >= r.left() && origin.x <= r.right() {
        p.vline(origin.x, r.y_range(), axis);
    }
    if origin.y >= r.top() && origin.y <= r.bottom() {
        p.hline(r.x_range(), origin.y, axis);
    }
}

/// Draw y = f(x) across the visible range, breaking the line at gaps and poles.
pub fn draw_curve(p: &Painter, r: Rect, v: &View, color: Color32, width: f32, f: &mut dyn FnMut(f64) -> f64) {
    let n = (r.width().max(1.0) * 2.0) as usize;
    let mut seg: Vec<Pos2> = Vec::with_capacity(n + 1);
    let stroke = Stroke::new(width, color);
    let flush = |seg: &mut Vec<Pos2>| {
        if seg.len() > 1 {
            p.add(Shape::line(std::mem::take(seg), stroke));
        } else {
            seg.clear();
        }
    };
    let (top, bottom) = (r.top(), r.bottom());
    for i in 0..=n {
        let px = r.left() + r.width() * i as f32 / n as f32;
        let x = v.cx + (px - r.center().x) as f64 * v.sx;
        let y = f(x);
        if !y.is_finite() {
            flush(&mut seg);
            continue;
        }
        let py = (r.center().y as f64 - (y - v.cy) / v.sy).clamp(-1.0e5, 1.0e5) as f32;
        if let Some(prev) = seg.last() {
            let jump = (prev.y < top && py > bottom) || (prev.y > bottom && py < top);
            if jump {
                flush(&mut seg);
            }
        }
        seg.push(pos2(px, py));
    }
    flush(&mut seg);
}

/// Draw y = f(x) as a dashed line.
pub fn draw_curve_dashed(p: &Painter, r: Rect, v: &View, color: Color32, width: f32, f: &mut dyn FnMut(f64) -> f64) {
    let n = (r.width().max(1.0) * 2.0) as usize;
    let mut seg: Vec<Pos2> = Vec::new();
    let flush = |seg: &mut Vec<Pos2>| {
        if seg.len() > 1 {
            p.add(Shape::dashed_line(seg, Stroke::new(width, color), 7.0, 5.0));
        }
        seg.clear();
    };
    let (top, bottom) = (r.top(), r.bottom());
    for i in 0..=n {
        let px = r.left() + r.width() * i as f32 / n as f32;
        let x = v.cx + (px - r.center().x) as f64 * v.sx;
        let y = f(x);
        if !y.is_finite() {
            flush(&mut seg);
            continue;
        }
        let py = (r.center().y as f64 - (y - v.cy) / v.sy).clamp(-1.0e5, 1.0e5) as f32;
        if let Some(prev) = seg.last() {
            if (prev.y < top && py > bottom) || (prev.y > bottom && py < top) {
                flush(&mut seg);
            }
        }
        seg.push(pos2(px, py));
    }
    flush(&mut seg);
}

/// Shade the region above (`above` = true) or below the curve y = f(x).
pub fn fill_region(p: &Painter, r: Rect, v: &View, fill: Color32, above: bool, f: &mut dyn FnMut(f64) -> f64) {
    let mut mesh = egui::Mesh::default();
    let mut joined = false;
    let edge = if above { r.top() - 2.0 } else { r.bottom() + 2.0 };
    let mut px = r.left().ceil();
    while px <= r.right() {
        let x = v.cx + (px - r.center().x) as f64 * v.sx;
        let y = f(x);
        if y.is_finite() {
            let py = (r.center().y as f64 - (y - v.cy) / v.sy).clamp(r.top() as f64 - 2.0, r.bottom() as f64 + 2.0) as f32;
            mesh.colored_vertex(pos2(px, edge), fill);
            mesh.colored_vertex(pos2(px, py), fill);
            if joined {
                let n = mesh.vertices.len() as u32;
                mesh.add_triangle(n - 4, n - 3, n - 2);
                mesh.add_triangle(n - 3, n - 2, n - 1);
            }
            joined = true;
        } else {
            joined = false;
        }
        px += 1.0;
    }
    p.add(Shape::mesh(mesh));
}

/// Draw a parametric path from sampled points.
pub fn draw_path(p: &Painter, r: Rect, v: &View, color: Color32, width: f32, pts: &[(f64, f64)]) {
    let stroke = Stroke::new(width, color);
    let mut seg: Vec<Pos2> = Vec::new();
    for (x, y) in pts {
        if x.is_finite() && y.is_finite() {
            seg.push(v.to_screen(r, *x, *y));
        } else if seg.len() > 1 {
            p.add(Shape::line(std::mem::take(&mut seg), stroke));
        } else {
            seg.clear();
        }
    }
    if seg.len() > 1 {
        p.add(Shape::line(seg, stroke));
    }
}

pub fn marker(p: &Painter, pos: Pos2, color: Color32, pal: &Pal) {
    p.circle(pos, 4.0, pal.surface, Stroke::new(1.8_f32, color));
}
