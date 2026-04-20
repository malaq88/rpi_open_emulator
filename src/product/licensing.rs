use crate::themes::ThemeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LicenseTier {
    #[default]
    Free,
    /// Reservado: desbloqueia scraping avançado, temas exclusivos, sync nuvem.
    Premium,
}

#[derive(Debug, Clone)]
pub struct FeatureFlags {
    pub tier: LicenseTier,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            tier: LicenseTier::Free,
        }
    }
}

impl FeatureFlags {
    pub fn premium_themes_unlocked(&self) -> bool {
        matches!(self.tier, LicenseTier::Premium)
    }

    pub fn advanced_auto_scraper(&self) -> bool {
        matches!(self.tier, LicenseTier::Premium)
    }

    pub fn cloud_sync(&self) -> bool {
        matches!(self.tier, LicenseTier::Premium)
    }

    /// Presets embutidos são gratuitos; IDs extra (loja / plugin) reservados para premium.
    pub fn theme_allowed(&self, _theme: ThemeId) -> bool {
        match self.tier {
            LicenseTier::Premium => true,
            LicenseTier::Free => true,
        }
    }
}
