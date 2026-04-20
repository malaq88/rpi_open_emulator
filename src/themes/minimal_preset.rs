use egui::{Color32, Rounding, Stroke, Visuals};

use super::palette::*;

pub fn visuals() -> (Visuals, Color32) {
    let accent = Color32::from_rgb(148, 163, 184);
    let mut v = Visuals::dark();
    v.dark_mode = true;
    v.override_text_color = Some(TEXT_MAIN);
    v.window_fill = BG_DEEP;
    v.panel_fill = BG_DEEP;
    v.extreme_bg_color = BG_DEEP;
    v.faint_bg_color = BG_CARD;
    v.code_bg_color = BG_CARD;
    v.window_stroke = Stroke::new(0.5, Color32::from_white_alpha(20));
    v.widgets.noninteractive.bg_stroke = Stroke::new(0.5, Color32::from_white_alpha(12));
    v.widgets.noninteractive.bg_fill = BG_CARD;
    v.widgets.inactive.bg_fill = BG_CARD;
    v.widgets.inactive.bg_stroke = Stroke::new(0.5, Color32::from_white_alpha(18));
    v.widgets.hovered.bg_fill = UI_CARD_ELEVATED;
    v.widgets.hovered.bg_stroke = Stroke::new(0.5, accent);
    v.widgets.active.bg_stroke = Stroke::new(0.5, TEXT_MAIN);
    v.selection.bg_fill = Color32::from_white_alpha(28);
    v.selection.stroke = Stroke::new(0.5, accent);
    let r = Rounding::same(6.0);
    v.widgets.noninteractive.rounding = r;
    v.widgets.inactive.rounding = r;
    v.widgets.hovered.rounding = r;
    v.widgets.active.rounding = r;
    v.widgets.open.rounding = r;
    v.window_rounding = Rounding::same(8.0);
    (v, ACCENT)
}
