//! Convert mode: units, number bases and physical constants.

use crate::engine::{self, Env, Value};
use crate::fmt::{self, Fmt};
use crate::help;
use crate::theme::{self, mix, Pal};
use crate::units::{self, CATEGORIES, CONSTANTS};
use crate::widgets;
use eframe::egui::{
    self, vec2, Align, Align2, ComboBox, CornerRadius, CursorIcon, Layout, RichText, ScrollArea, Sense, Ui,
};

pub struct ConvState {
    pub sub: usize,
    pub cat: usize,
    pub from: usize,
    pub value: String,
    base_text: [String; 4],
    base_error: Option<usize>,
}

impl Default for ConvState {
    fn default() -> Self {
        ConvState {
            sub: 0,
            cat: 0,
            from: 0,
            value: "1".into(),
            base_text: Default::default(),
            base_error: None,
        }
    }
}

const BASES: [(&str, u32); 4] = [("Decimal", 10), ("Hexadecimal", 16), ("Octal", 8), ("Binary", 2)];

/// Returns text to insert into the Calculate input, if the user picked a constant.
pub fn ui(ui: &mut Ui, st: &mut ConvState, env: &mut Env, f: Fmt, pal: &Pal) -> Option<String> {
    let mut picked = None;
    widgets::segmented(ui, pal, &mut st.sub, &["Units", "Number base", "Constants"]);
    ui.add_space(14.0);
    match st.sub {
        0 => units_ui(ui, st, env, pal),
        1 => base_ui(ui, st, pal),
        _ => picked = constants_ui(ui, f, pal),
    }
    picked
}

fn units_ui(ui: &mut Ui, st: &mut ConvState, env: &mut Env, pal: &Pal) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
        for (i, c) in CATEGORIES.iter().enumerate() {
            if widgets::tab(ui, pal, c.name, st.cat == i) && st.cat != i {
                st.cat = i;
                st.from = 0;
            }
        }
    });
    ui.add_space(12.0);
    let cat = &CATEGORIES[st.cat];
    st.from = st.from.min(cat.units.len() - 1);
    ui.horizontal(|ui| {
        widgets::field(ui, pal, &mut st.value, "Value", 150.0).on_hover_text(help::NUMBER);
        ui.spacing_mut().interact_size.y = 32.0;
        ComboBox::from_id_salt("unit_from")
            .width(200.0)
            .selected_text(format!("{}  ({})", cat.units[st.from].name, cat.units[st.from].symbol))
            .show_ui(ui, |ui| {
                for (i, u) in cat.units.iter().enumerate() {
                    ui.selectable_value(&mut st.from, i, format!("{}  ({})", u.name, u.symbol));
                }
            });
    });
    ui.add_space(12.0);
    let value = match engine::field_value(&st.value, env) {
        Ok(Some(v)) => v,
        Ok(None) => {
            widgets::dim(ui, pal, "Enter a value to convert. Expressions work here too.");
            return;
        }
        Err(e) => {
            widgets::error(ui, pal, e);
            return;
        }
    };
    let from = &cat.units[st.from];
    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.set_max_width(520.0);
        widgets::card(ui, pal, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for (i, u) in cat.units.iter().enumerate() {
                if i == st.from {
                    continue;
                }
                let v = units::convert(value, from, u);
                let resp = ui
                    .horizontal(|ui| {
                        ui.set_min_height(26.0);
                        ui.label(RichText::new(u.name).color(pal.dim).size(13.5));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let (r, _) = ui.allocate_exact_size(vec2(54.0, 20.0), Sense::hover());
                            ui.painter().text(
                                r.left_center() + vec2(6.0, 0.0),
                                Align2::LEFT_CENTER,
                                u.symbol,
                                theme::regular(13.0),
                                pal.dim,
                            );
                            ui.add(egui::Label::new(RichText::new(fmt::short(v, 10)).size(15.0)).selectable(true));
                        });
                    })
                    .response;
                let _ = resp;
            }
        });
    });
}

fn base_ui(ui: &mut Ui, st: &mut ConvState, pal: &Pal) {
    let mut edited: Option<usize> = None;
    for (i, (name, _)) in BASES.iter().enumerate() {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(110.0, 32.0), Sense::hover());
            ui.painter().text(r.left_center(), Align2::LEFT_CENTER, *name, theme::regular(13.5), pal.dim);
            let w = (ui.available_width() - 40.0).min(520.0);
            if widgets::field(ui, pal, &mut st.base_text[i], "", w).on_hover_text(help::BASE).changed() {
                edited = Some(i);
            }
            if st.base_error == Some(i) {
                widgets::error(ui, pal, "Not valid in this base");
            }
        });
    }
    if let Some(i) = edited {
        st.base_error = None;
        if st.base_text[i].trim().is_empty() {
            st.base_text = Default::default();
        } else {
            match fmt::from_base(&st.base_text[i], BASES[i].1) {
                Some(v) => {
                    for (j, (_, radix)) in BASES.iter().enumerate() {
                        if j != i {
                            st.base_text[j] = fmt::to_base(v, *radix);
                        }
                    }
                }
                None => st.base_error = Some(i),
            }
        }
    }
    ui.add_space(12.0);
    widgets::dim(
        ui,
        pal,
        "Type in any row and the others follow. Whole numbers up to 64 bits; negatives use two's complement.",
    );
    ui.add_space(2.0);
    widgets::dim(
        ui,
        pal,
        "In Calculate you can write 0xFF, 0b1010 and 0o17, and use and( ) or( ) xor( ) not( ) shl( ) shr( ).",
    );
}

fn constants_ui(ui: &mut Ui, f: Fmt, pal: &Pal) -> Option<String> {
    let mut picked = None;
    widgets::dim(ui, pal, "Click a constant to use its value in Calculate.");
    ui.add_space(6.0);
    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.set_max_width(640.0);
        ui.spacing_mut().item_spacing.y = 2.0;
        for c in CONSTANTS {
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
            let hover = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.10);
            if hover > 0.0 {
                ui.painter().rect_filled(rect, CornerRadius::same(7), mix(pal.bg, pal.key, hover));
            }
            let p = ui.painter();
            p.text(rect.left_center() + vec2(10.0, 0.0), Align2::LEFT_CENTER, c.symbol, theme::medium(14.5), pal.text);
            p.text(rect.left_center() + vec2(58.0, 0.0), Align2::LEFT_CENTER, c.name, theme::regular(13.5), pal.dim);
            p.text(
                rect.right_center() - vec2(112.0, 0.0),
                Align2::RIGHT_CENTER,
                fmt::real(c.value, f),
                theme::regular(14.5),
                pal.text,
            );
            p.text(rect.right_center() - vec2(100.0, 0.0), Align2::LEFT_CENTER, c.unit, theme::regular(12.5), pal.dim);
            if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                picked = Some(fmt::plain(&Value::real(c.value)));
            }
        }
    });
    picked
}
