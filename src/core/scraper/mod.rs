//! Base para scraping de metadados (plugins podem registar fontes).

use crate::config::LauncherConfig;
use crate::core::library::GameEntry;

/// Fonte de metadados plugável (implementações futuras: ScreenScraper, IGDB, etc.).
pub trait MetadataSource: Send + Sync {
    fn id(&self) -> &'static str;
    /// Retorna descrição curta para UI / logs.
    fn label(&self) -> &'static str;
}

/// Pipeline offline atual (capas locais + títulos derivados do ficheiro).
pub struct OfflineLibrarySource;

impl MetadataSource for OfflineLibrarySource {
    fn id(&self) -> &'static str {
        "offline-library"
    }

    fn label(&self) -> &'static str {
        "Metadados locais (cache offline)"
    }
}

/// Ponto de extensão para orquestrar várias fontes (premium pode adicionar APIs).
pub struct ScraperPipeline {
    pub sources: Vec<Box<dyn MetadataSource>>,
}

impl Default for ScraperPipeline {
    fn default() -> Self {
        Self {
            sources: vec![Box::new(OfflineLibrarySource)],
        }
    }
}

impl ScraperPipeline {
    pub fn describe(&self) -> String {
        self.sources
            .iter()
            .map(|s| format!("{} ({})", s.label(), s.id()))
            .collect::<Vec<_>>()
            .join(" · ")
    }

    /// Placeholder: no futuro, fontes plugáveis preenchem `GameEntry` remotamente.
    pub fn refresh_all(
        &mut self,
        _games: &[GameEntry],
        _config: &LauncherConfig,
    ) -> anyhow::Result<usize> {
        Ok(0)
    }
}
