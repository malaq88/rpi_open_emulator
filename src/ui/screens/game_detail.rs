use std::path::PathBuf;

use egui::{Frame, Margin, RichText, Rounding, Stroke, Ui};

use crate::core::library::GameEntry;
use crate::theme;

pub enum DetailCommand {
    Launch(GameEntry),
    Favorite(PathBuf, bool),
    Back,
}

pub fn render_game_detail(
    ui: &mut Ui,
    game: &GameEntry,
) -> Option<DetailCommand> {
    let mut cmd = None;

    ui.horizontal(|ui| {
        if theme::outline_button(ui, "← Voltar", theme::TEXT_DIM).clicked() {
            cmd = Some(DetailCommand::Back);
        }
    });
    ui.add_space(12.0);

    Frame::none()
        .fill(theme::BG_CARD)
        .rounding(Rounding::same(14.0))
        .stroke(Stroke::new(1.0, theme::accent_for_system(&game.system_key).linear_multiply(0.4)))
        .inner_margin(Margin::same(20.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new(&game.title)
                    .strong()
                    .size(24.0)
                    .color(theme::TEXT_MAIN),
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new(game.system_key.to_ascii_uppercase())
                    .strong()
                    .color(theme::accent_for_system(&game.system_key)),
            );
            ui.add_space(16.0);

            let year = game
                .release_year
                .map(|y| y.to_string())
                .unwrap_or_else(|| "—".to_string());
            let genre = game.genre.as_deref().unwrap_or("—");
            let dev = game.developer.as_deref().unwrap_or("—");
            ui.label(
                RichText::new(format!("Ano: {}   Género: {}", year, genre))
                    .color(theme::TEXT_DIM),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("Desenvolvedor: {}", dev))
                    .color(theme::TEXT_DIM),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("Fonte metadados: {}", game.metadata_source))
                    .small()
                    .color(theme::TEXT_DIM),
            );
            ui.add_space(12.0);

            if let Some(desc) = game.description.as_ref() {
                ui.label(RichText::new(desc).color(theme::TEXT_MAIN));
            } else {
                ui.label(
                    RichText::new("Sem descrição — futuros scrapers/plugins podem enriquecer esta ficha.")
                        .italics()
                        .color(theme::TEXT_DIM),
                );
            }

            ui.add_space(20.0);
            ui.horizontal(|ui| {
                if theme::action_button(ui, "  Jogar  ").clicked() {
                    cmd = Some(DetailCommand::Launch(game.clone()));
                }
                ui.add_space(10.0);
                let fav_label = if game.is_favorite {
                    "★ Remover favorito"
                } else {
                    "☆ Marcar favorito"
                };
                if theme::outline_button(ui, fav_label, theme::WARM).clicked() {
                    cmd = Some(DetailCommand::Favorite(game.path.clone(), !game.is_favorite));
                }
            });
        });

    cmd
}
