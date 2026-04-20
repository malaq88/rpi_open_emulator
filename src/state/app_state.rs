use std::path::PathBuf;

use crate::core::library::GameEntry;

use super::filter_state::FilterState;

/// Vista principal da aplicação (navegação de alto nível).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppView {
    Dashboard,
    Library,
    GameDetail(PathBuf),
    Settings,
}

/// Estado central exposto ao UI (alinhado à visão de “hub” de jogos).
#[derive(Debug, Clone)]
pub struct AppState {
    pub games: Vec<GameEntry>,
    pub recent: Vec<GameEntry>,
    pub recently_added: Vec<GameEntry>,
    pub most_played: Vec<GameEntry>,
    pub filter: FilterState,
    pub selected_view: AppView,
}

impl AppState {
    pub fn new(
        games: Vec<GameEntry>,
        recent: Vec<GameEntry>,
        recently_added: Vec<GameEntry>,
        most_played: Vec<GameEntry>,
    ) -> Self {
        Self {
            games,
            recent,
            recently_added,
            most_played,
            filter: FilterState::default(),
            selected_view: AppView::Dashboard,
        }
    }
}
