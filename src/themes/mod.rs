//! Temas visuais (presets embutidos + gancho para temas custom em `themes/custom`).

pub mod custom;
pub mod default_preset;
pub mod minimal_preset;
pub mod neon_preset;
pub mod palette;

use egui::{Color32, Context, RichText, Rounding, Stroke, Style, Ui};

pub use palette::*;

/// Identificador de preset (troca em runtime; premium pode desbloquear extras).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeId {
    #[default]
    DefaultDark,
    Neon,
    Minimal,
}

impl ThemeId {
    pub const ALL: [ThemeId; 3] = [ThemeId::DefaultDark, ThemeId::Neon, ThemeId::Minimal];

    pub fn label(self) -> &'static str {
        match self {
            ThemeId::DefaultDark => "Escuro (Steam-like)",
            ThemeId::Neon => "Neon",
            ThemeId::Minimal => "Minimal",
        }
    }
}

pub fn apply(ctx: &Context, id: ThemeId) {
    let (mut v, accent_main) = match id {
        ThemeId::DefaultDark => default_preset::visuals(),
        ThemeId::Neon => neon_preset::visuals(),
        ThemeId::Minimal => minimal_preset::visuals(),
    };
    v.hyperlink_color = accent_main;
    ctx.set_visuals(v);
    ctx.style_mut(|style| apply_style(style));
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
    style
        .text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
}

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

pub fn action_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).strong().color(Color32::BLACK))
            .fill(ACTION)
            .rounding(Rounding::same(8.0)),
    )
}

pub fn outline_button(ui: &mut Ui, label: &str, border: Color32) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).color(TEXT_MAIN))
            .fill(Color32::from_rgb(36, 48, 66))
            .stroke(Stroke::new(1.5, border))
            .rounding(Rounding::same(8.0)),
    )
}
