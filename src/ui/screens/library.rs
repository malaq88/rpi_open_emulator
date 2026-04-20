use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use egui::{Color32, Context, Frame, Margin, RichText, Rounding, ScrollArea, Stroke, Ui};

use crate::core::library::GameEntry;
use crate::state::filter_state::{self, FilterState};
use crate::theme;
use crate::ui::components::game_card;

pub use crate::ui::components::sidebar::SidebarCounts;

pub enum LibraryCommand {
    Launch(GameEntry),
    Favorite(PathBuf, bool),
    OpenDetail(PathBuf),
}

pub fn build_sidebar_counts(games: &[GameEntry], config: &crate::config::LauncherConfig) -> SidebarCounts {
    let total = games.len();
    let favorites = games.iter().filter(|g| g.is_favorite).count();
    let recent = games.iter().filter(|g| g.last_played_at.is_some()).count();
    let mut per: HashMap<String, usize> = HashMap::new();
    for g in games {
        let k = g.system_key.to_ascii_lowercase();
        *per.entry(k).or_insert(0) += 1;
    }
    let mut keys: Vec<String> = config.systems.keys().cloned().collect();
    keys.sort();
    let systems = keys
        .into_iter()
        .map(|k| (k.clone(), *per.get(&k.to_ascii_lowercase()).unwrap_or(&0)))
        .collect();
    SidebarCounts {
        total,
        favorites,
        recent,
        systems,
    }
}

/// Grelha da biblioteca (filtros e busca vêm do header global em `main`).
pub fn render_library_grid(
    ui: &mut Ui,
    ctx: &Context,
    games: &[GameEntry],
    recent: &[GameEntry],
    filter: &FilterState,
    cover_cache: &mut HashMap<PathBuf, egui::TextureHandle>,
) -> Vec<LibraryCommand> {
    let mut cmds = Vec::new();
    let recent_paths: HashSet<PathBuf> = recent.iter().map(|g| g.path.clone()).collect();
    let filtered = filter_state::filter_games(games, filter, &recent_paths);

    Frame::none()
        .fill(theme::BG_CARD)
        .rounding(Rounding::same(10.0))
        .stroke(Stroke::new(1.0, Color32::from_rgb(60, 72, 105)))
        .inner_margin(Margin::same(12.0))
        .show(ui, |ui| {
            ui.set_min_height(ui.available_height());
            let body_w = ui.available_width();
            let scroll_h = ui.available_height();
            let gap = 12.0;
            let min_card = 148.0;
            let cols = ((body_w + gap) / (min_card + gap)).floor().max(1.0) as usize;
            let card_w = (body_w - gap * (cols.saturating_sub(1)) as f32) / cols as f32;

            ScrollArea::vertical()
                .id_source("library_game_grid_scroll")
                .max_height(scroll_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if filtered.is_empty() {
                        ui.label(
                            RichText::new("Nenhum jogo corresponde ao filtro ou à busca.")
                                .color(theme::TEXT_DIM)
                                .italics(),
                        );
                        return;
                    }

                    let mut i = 0;
                    while i < filtered.len() {
                        ui.horizontal(|ui| {
                            for _ in 0..cols {
                                if i >= filtered.len() {
                                    break;
                                }
                                let game = &filtered[i];
                                ui.vertical(|ui| {
                                    ui.set_width(card_w);
                                    let out = game_card::render_game_card(
                                        ui,
                                        ctx,
                                        game,
                                        cover_cache,
                                        card_w,
                                        true,
                                    );
                                    if out.play {
                                        cmds.push(LibraryCommand::Launch(game.clone()));
                                    }
                                    if let Some(v) = out.favorite_toggle {
                                        cmds.push(LibraryCommand::Favorite(game.path.clone(), v));
                                    }
                                    if out.open_details {
                                        cmds.push(LibraryCommand::OpenDetail(game.path.clone()));
                                    }
                                });
                                i += 1;
                            }
                        });
                        ui.add_space(gap);
                    }
                });
        });

    cmds
}
