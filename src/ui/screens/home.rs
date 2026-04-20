use std::collections::HashMap;
use std::path::PathBuf;

use egui::{Context, Frame, Margin, RichText, Rounding, ScrollArea, Stroke, TextureHandle, Ui};

use crate::config::LauncherConfig;
use crate::core::library::GameEntry;
use crate::state::filter_state::FilterState;
use crate::theme;
use crate::ui::components::game_card;

pub enum HomeCommand {
    Launch(GameEntry),
    Favorite(PathBuf, bool),
    OpenDetail(PathBuf),
}

fn filter_by_search(games: &[GameEntry], filter: &FilterState) -> Vec<GameEntry> {
    games
        .iter()
        .filter(|g| filter.matches_search(g))
        .cloned()
        .collect()
}

fn carousel_row(
    ui: &mut Ui,
    ctx: &Context,
    title: &str,
    games: &[GameEntry],
    cover_cache: &mut HashMap<PathBuf, TextureHandle>,
    card_w: f32,
    cmds: &mut Vec<HomeCommand>,
) {
    theme::section_heading(ui, title, theme::ACCENT);
    if games.is_empty() {
        ui.label(
            RichText::new("Sem entradas por agora.")
                .italics()
                .color(theme::TEXT_DIM),
        );
        ui.add_space(10.0);
        return;
    }
    ScrollArea::horizontal()
        .id_source(egui::Id::new("carousel").with(title))
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for g in games {
                    ui.vertical(|ui| {
                        ui.set_width(card_w);
                        let out = game_card::render_game_card(ui, ctx, g, cover_cache, card_w, true);
                        if out.play {
                            cmds.push(HomeCommand::Launch(g.clone()));
                        }
                        if let Some(v) = out.favorite_toggle {
                            cmds.push(HomeCommand::Favorite(g.path.clone(), v));
                        }
                        if out.open_details {
                            cmds.push(HomeCommand::OpenDetail(g.path.clone()));
                        }
                    });
                    ui.add_space(14.0);
                }
            });
        });
    ui.add_space(16.0);
}

/// Home: carrosséis horizontais estilo consola / Steam.
pub fn render_home(
    ui: &mut Ui,
    ctx: &Context,
    games: &[GameEntry],
    recent: &[GameEntry],
    recently_added: &[GameEntry],
    most_played: &[GameEntry],
    config: &LauncherConfig,
    filter: &FilterState,
    cover_cache: &mut HashMap<PathBuf, TextureHandle>,
) -> Vec<HomeCommand> {
    let mut cmds = Vec::new();
    let card_w = 148.0;
    let recent_f = filter_by_search(recent, filter);
    let added_f = filter_by_search(recently_added, filter);
    let played_f = filter_by_search(most_played, filter);

    ui.label(
        RichText::new("Início")
            .strong()
            .size(22.0)
            .color(theme::TEXT_MAIN),
    );
    ui.add_space(4.0);
    ui.label(
        RichText::new("Retome sessões, descubra novidades e os mais jogados.")
            .color(theme::TEXT_DIM),
    );
    ui.add_space(18.0);

    let continue_game = config
        .history
        .last_game_path
        .as_ref()
        .and_then(|p| games.iter().find(|g| &g.path == p).cloned())
        .or_else(|| recent.first().cloned());

    if let Some(ref g) = continue_game {
        Frame::none()
            .fill(theme::UI_CARD_ELEVATED)
            .rounding(Rounding::same(14.0))
            .stroke(Stroke::new(1.0, theme::ACCENT.linear_multiply(0.55)))
            .inner_margin(Margin::symmetric(20.0, 16.0))
            .show(ui, |ui| {
                theme::section_heading(ui, "Continuar a jogar", theme::ACCENT);
                ui.horizontal(|ui| {
                    let out = game_card::render_game_card(ui, ctx, g, cover_cache, 158.0, true);
                    if out.play {
                        cmds.push(HomeCommand::Launch(g.clone()));
                    }
                    if let Some(v) = out.favorite_toggle {
                        cmds.push(HomeCommand::Favorite(g.path.clone(), v));
                    }
                    if out.open_details {
                        cmds.push(HomeCommand::OpenDetail(g.path.clone()));
                    }
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(&g.title)
                                .strong()
                                .size(18.0)
                                .color(theme::TEXT_MAIN),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(g.system_key.to_ascii_uppercase())
                                .color(theme::accent_for_system(&g.system_key)),
                        );
                    });
                });
            });
        ui.add_space(20.0);
    }

    carousel_row(
        ui,
        ctx,
        "Adicionados recentemente",
        &added_f,
        cover_cache,
        card_w,
        &mut cmds,
    );
    carousel_row(
        ui,
        ctx,
        "Mais jogados",
        &played_f,
        cover_cache,
        card_w,
        &mut cmds,
    );
    carousel_row(
        ui,
        ctx,
        "Recentes",
        &recent_f,
        cover_cache,
        card_w,
        &mut cmds,
    );

    cmds
}
