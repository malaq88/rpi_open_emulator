//! Tema visual “console” — paleta escura com acentos coloridos (Fase A).

use egui::{Color32, Context, RichText, Rounding, Stroke, Style, Ui, Visuals};

/// LED / destaque principal (ciano-esverdeado).
pub const ACCENT: Color32 = Color32::from_rgb(0, 214, 178);
/// Botão de ação principal (laranja retro).
pub const ACTION: Color32 = Color32::from_rgb(255, 122, 61);
/// Aviso / favorito.
pub const WARM: Color32 = Color32::from_rgb(255, 204, 102);

pub const BG_DEEP: Color32 = Color32::from_rgb(12, 14, 22);
pub const BG_PANEL: Color32 = Color32::from_rgb(22, 26, 40);
pub const BG_CARD: Color32 = Color32::from_rgb(30, 36, 54);
pub const TEXT_DIM: Color32 = Color32::from_rgb(160, 170, 195);
pub const TEXT_MAIN: Color32 = Color32::from_rgb(230, 235, 245);

pub fn accent_for_system(system_key: &str) -> Color32 {
    let key = system_key.to_ascii_lowercase();
    match key.as_str() {
        "nes" => Color32::from_rgb(220, 58, 58),
        "snes" => Color32::from_rgb(140, 90, 220),
        "gba" => Color32::from_rgb(80, 120, 255),
        "gb" | "gbc" => Color32::from_rgb(90, 160, 90),
        "genesis" | "mastersystem" | "gamegear" | "segacd" | "sega32x" | "sg1000" => {
            Color32::from_rgb(80, 200, 120)
        }
        "n64" => Color32::from_rgb(40, 120, 255),
        "nds" => Color32::from_rgb(60, 140, 220),
        "psx" | "psp" | "ps2" => Color32::from_rgb(180, 180, 220),
        "dreamcast" | "saturn" => Color32::from_rgb(255, 140, 80),
        "arcade" | "neogeo" | "fbneo" => Color32::from_rgb(220, 100, 60),
        "gamecube" | "wii" => Color32::from_rgb(100, 160, 255),
        "pce" | "supergrafx" | "pcfx" => Color32::from_rgb(200, 80, 140),
        _ => {
            let n = key.bytes().fold(0u32, |a, c| a.wrapping_mul(31).wrapping_add(c as u32));
            let r = 90 + (n % 110) as u8;
            let g = 110 + ((n >> 7) % 90) as u8;
            let b = 130 + ((n >> 14) % 90) as u8;
            Color32::from_rgb(r, g, b)
        }
    }
}

fn console_visuals() -> Visuals {
    let mut v = Visuals::dark();
    v.dark_mode = true;

    v.override_text_color = Some(TEXT_MAIN);

    v.window_fill = BG_DEEP;
    v.panel_fill = BG_PANEL;
    v.extreme_bg_color = BG_DEEP;
    v.faint_bg_color = BG_CARD;
    v.code_bg_color = BG_CARD;

    v.window_stroke = Stroke::new(1.0, Color32::from_rgb(55, 65, 95));
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(50, 58, 85));
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_DIM);
    v.widgets.noninteractive.bg_fill = BG_CARD;

    v.widgets.inactive.bg_fill = Color32::from_rgb(38, 44, 64);
    v.widgets.inactive.weak_bg_fill = Color32::from_rgb(32, 38, 56);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(60, 72, 105));
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_MAIN);

    v.widgets.hovered.bg_fill = Color32::from_rgb(48, 56, 82);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(42, 50, 75);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_MAIN);

    v.widgets.active.bg_fill = Color32::from_rgb(55, 65, 95);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(48, 58, 88);
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACTION);
    v.widgets.active.fg_stroke = Stroke::new(1.5, TEXT_MAIN);

    v.widgets.open.bg_fill = v.widgets.active.bg_fill;
    v.widgets.open.fg_stroke = v.widgets.active.fg_stroke;

    v.selection.bg_fill = Color32::from_rgba_premultiplied(0, 214, 178, 55);
    v.selection.stroke = Stroke::new(1.0, ACCENT);

    v.hyperlink_color = ACCENT;
    v.warn_fg_color = WARM;
    v.error_fg_color = Color32::from_rgb(255, 100, 120);

    let r = Rounding::same(7.0);
    v.widgets.noninteractive.rounding = r;
    v.widgets.inactive.rounding = r;
    v.widgets.hovered.rounding = r;
    v.widgets.active.rounding = r;
    v.widgets.open.rounding = r;

    v.window_rounding = Rounding::same(10.0);
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

    v
}

fn apply_style(style: &mut Style) {
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 7.0);
    style.spacing.window_margin = egui::Margin::same(12.0);
    style.spacing.menu_margin = egui::Margin::same(8.0);

    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::proportional(22.0),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::proportional(15.0),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::proportional(12.0),
    );
}

pub fn apply(ctx: &Context) {
    ctx.set_visuals(console_visuals());
    ctx.style_mut(apply_style);
}

/// Titulo de secao com barra colorida (estilo “console menu”).
pub fn section_heading(ui: &mut Ui, title: &str, accent: Color32) {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        let bar_height = 20.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(5.0, bar_height), egui::Sense::hover());
        ui.painter().rect_filled(rect, Rounding::same(2.0), accent);
        ui.add_space(8.0);
        ui.label(
            RichText::new(title)
                .strong()
                .size(17.0)
                .color(TEXT_MAIN),
        );
    });
    ui.add_space(4.0);
}

/// Botao de acao principal (laranja).
pub fn action_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).strong().color(Color32::BLACK))
            .fill(ACTION)
            .rounding(Rounding::same(8.0)),
    )
}

/// Botao secundario com borda em destaque.
pub fn outline_button(ui: &mut Ui, label: &str, border: Color32) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).color(TEXT_MAIN))
            .fill(Color32::from_rgb(36, 42, 62))
            .stroke(Stroke::new(1.5, border))
            .rounding(Rounding::same(8.0)),
    )
}
