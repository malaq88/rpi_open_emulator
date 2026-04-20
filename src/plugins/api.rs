//! API pública mínima para plugins de terceiros (carregamento dinâmico = futuro).

use crate::config::LauncherConfig;
use crate::core::library::GameEntry;
use crate::themes::ThemeId;

/// Registo de extensões (futuro: DLL / WASM / scripts).
#[derive(Default)]
pub struct PluginHost {
    scrapers: Vec<String>,
}

impl PluginHost {
    pub fn register_scraper_id(&mut self, id: impl Into<String>) {
        self.scrapers.push(id.into());
    }

    pub fn list_scraper_ids(&self) -> &[String] {
        &self.scrapers
    }
}

/// Contrato para fontes de metadados remotas (implementação futura em crate separado).
pub trait ScraperPlugin: Send + Sync {
    fn id(&self) -> &'static str;
    fn refresh_game(&self, _game: &GameEntry, _config: &LauncherConfig) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Temas entregues como plugin (premium / loja).
pub trait ThemePlugin: Send + Sync {
    fn id(&self) -> &'static str;
    fn theme_id(&self) -> ThemeId;
}
