use egui::{Color32, Rounding, Stroke, Visuals};

use super::palette::*;

pub fn visuals() -> (Visuals, Color32) {
    let mut v = Visuals::dark();
    v.dark_mode = true;
    v.override_text_color = Some(TEXT_MAIN);
    v.window_fill = BG_DEEP;
    v.panel_fill = BG_PANEL;
    v.extreme_bg_color = BG_DEEP;
    v.faint_bg_color = BG_CARD;
    v.code_bg_color = BG_CARD;
    v.window_stroke = Stroke::new(1.0, Color32::from_rgb(51, 65, 85));
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(51, 65, 85));
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_DIM);
    v.widgets.noninteractive.bg_fill = BG_CARD;
    v.widgets.inactive.bg_fill = Color32::from_rgb(41, 53, 72);
    v.widgets.inactive.weak_bg_fill = Color32::from_rgb(36, 48, 66);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(71, 85, 105));
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_MAIN);
    v.widgets.hovered.bg_fill = Color32::from_rgb(51, 65, 85);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(45, 58, 78);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_MAIN);
    v.widgets.active.bg_fill = Color32::from_rgb(56, 72, 96);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(48, 62, 84);
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACTION);
    v.widgets.active.fg_stroke = Stroke::new(1.5, TEXT_MAIN);
    v.widgets.open.bg_fill = v.widgets.active.bg_fill;
    v.widgets.open.fg_stroke = v.widgets.active.fg_stroke;
    v.selection.bg_fill = Color32::from_rgba_premultiplied(56, 189, 248, 45);
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.warn_fg_color = WARM;
    v.error_fg_color = Color32::from_rgb(248, 113, 113);
    let r = Rounding::same(10.0);
    v.widgets.noninteractive.rounding = r;
    v.widgets.inactive.rounding = r;
    v.widgets.hovered.rounding = r;
    v.widgets.active.rounding = r;
    v.widgets.open.rounding = r;
    v.window_rounding = Rounding::same(12.0);
    v.menu_rounding = Rounding::same(8.0);
    v.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 10.0),
        blur: 24.0,
        spread: 0.0,
        color: Color32::from_black_alpha(120),
    };
    v.popup_shadow = egui::epaint::Shadow {
        offset: egui::vec2(6.0, 10.0),
        blur: 16.0,
        spread: 0.0,
        color: Color32::from_black_alpha(140),
    };
    (v, ACCENT)
}
