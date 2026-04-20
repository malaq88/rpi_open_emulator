use std::collections::HashSet;

use crate::core::library::GameEntry;

/// Secção ativa na barra lateral (biblioteca filtrada).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarSection {
    /// Toda a biblioteca.
    All,
    /// Apenas favoritos.
    Favorites,
    /// Jogados recentemente.
    Recent,
    /// Uma consola específica (chave = pasta do sistema, ex.: `snes`).
    Console(String),
}

/// Estado reativo de filtro (sidebar + busca).
#[derive(Debug, Clone)]
pub struct FilterState {
    pub section: SidebarSection,
    pub search_query: String,
}

impl Default for FilterState {
    fn default() -> Self {
        Self {
            section: SidebarSection::All,
            search_query: String::new(),
        }
    }
}

impl FilterState {
    /// Busca por substring + correspondência “fuzzy” leve (subsequência ordenada).
    pub fn matches_search(&self, game: &GameEntry) -> bool {
        let q = self.search_query.trim();
        if q.is_empty() {
            return true;
        }
        let ql = q.to_ascii_lowercase();
        let blob = format!(
            "{} {} {}",
            game.title, game.file_name, game.system_key
        );
        if blob.to_ascii_lowercase().contains(&ql) {
            return true;
        }
        [game.title.as_str(), game.file_name.as_str(), game.system_key.as_str()]
            .iter()
            .any(|field| subsequence_fuzzy_match(field, q))
    }
}

/// Padrão como subsequência de `choice` (ex.: "mrio" → "Mario"), case-insensitive.
fn subsequence_fuzzy_match(choice: &str, pattern: &str) -> bool {
    let c: Vec<char> = choice.to_ascii_lowercase().chars().collect();
    let p: Vec<char> = pattern.to_ascii_lowercase().chars().collect();
    if p.is_empty() {
        return true;
    }
    let mut ci = 0usize;
    for ch in &p {
        while ci < c.len() && &c[ci] != ch {
            ci += 1;
        }
        if ci >= c.len() {
            return false;
        }
        ci += 1;
    }
    true
}

/// Filtra jogos conforme secção da sidebar e texto de busca.
pub fn filter_games(
    games: &[GameEntry],
    filter: &FilterState,
    recent_paths: &HashSet<std::path::PathBuf>,
) -> Vec<GameEntry> {
    games
        .iter()
        .filter(|g| match &filter.section {
            SidebarSection::All => true,
            SidebarSection::Favorites => g.is_favorite,
            SidebarSection::Recent => recent_paths.contains(&g.path),
            SidebarSection::Console(key) => g.system_key.eq_ignore_ascii_case(key),
        })
        .filter(|g| filter.matches_search(g))
        .cloned()
        .collect()
}
