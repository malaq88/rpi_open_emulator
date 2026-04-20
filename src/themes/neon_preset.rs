use egui::{Color32, Rounding, Stroke, Visuals};

use super::palette::*;

pub fn visuals() -> (Visuals, Color32) {
    let accent = Color32::from_rgb(217, 70, 239);
    let mut v = Visuals::dark();
    v.dark_mode = true;
    v.override_text_color = Some(TEXT_MAIN);
    v.window_fill = Color32::from_rgb(12, 10, 28);
    v.panel_fill = Color32::from_rgb(22, 16, 42);
    v.extreme_bg_color = v.window_fill;
    v.faint_bg_color = Color32::from_rgb(36, 24, 58);
    v.code_bg_color = v.faint_bg_color;
    v.window_stroke = Stroke::new(1.0, accent.linear_multiply(0.35));
    v.widgets.noninteractive.bg_fill = Color32::from_rgb(32, 22, 52);
    v.widgets.inactive.bg_fill = Color32::from_rgb(40, 26, 64);
    v.widgets.hovered.bg_fill = Color32::from_rgb(52, 32, 82);
    v.widgets.hovered.bg_stroke = Stroke::new(1.2, accent);
    v.widgets.active.bg_stroke = Stroke::new(1.2, Color32::from_rgb(34, 211, 238));
    v.selection.bg_fill = accent.linear_multiply(0.25);
    v.selection.stroke = Stroke::new(1.0, accent);
    let r = Rounding::same(12.0);
    v.widgets.noninteractive.rounding = r;
    v.widgets.inactive.rounding = r;
    v.widgets.hovered.rounding = r;
    v.widgets.active.rounding = r;
    v.widgets.open.rounding = r;
    v.window_rounding = Rounding::same(14.0);
    (v, accent)
}
