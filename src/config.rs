use std::{
    collections::HashMap,
    env,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    pub retroarch: RetroArchConfig,
    pub library: LibraryConfig,
    #[serde(default)]
    pub systems: HashMap<String, SystemConfig>,
    #[serde(default)]
    pub history: HistoryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetroArchConfig {
    pub binary_path: PathBuf,
    pub cores_dir: PathBuf,
    #[serde(default)]
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryConfig {
    pub roms_dir: PathBuf,
    pub bios_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub default_core: String,
    #[serde(default)]
    pub accepted_extensions: Vec<String>,
    #[serde(default)]
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistoryConfig {
    pub last_game_path: Option<PathBuf>,
}

impl LauncherConfig {
    pub fn load_or_create(config_path: &Path) -> anyhow::Result<Self> {
        if config_path.exists() {
            let mut loaded = Self::load_from_file(config_path)?;
            loaded.migrate_legacy_paths_if_needed(config_path)?;
            loaded.migrate_retroarch_defaults_if_needed(config_path)?;
            return Ok(loaded);
        }

        let default = Self::default_template();
        default.save_to_file(config_path)?;
        Ok(default)
    }

    pub fn load_from_file(config_path: &Path) -> anyhow::Result<Self> {
        let raw = fs::read_to_string(config_path)
            .with_context(|| format!("falha ao ler configuracao em {}", config_path.display()))?;
        let parsed: Self = toml::from_str(&raw)
            .with_context(|| format!("falha ao parsear TOML em {}", config_path.display()))?;
        Ok(parsed)
    }

    pub fn save_to_file(&self, config_path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("falha ao criar diretorio de config {}", parent.display())
            })?;
        }

        let encoded =
            toml::to_string_pretty(self).context("falha ao serializar configuracao TOML")?;
        fs::write(config_path, encoded)
            .with_context(|| format!("falha ao gravar configuracao em {}", config_path.display()))
    }

    pub fn default_template() -> Self {
        let home_dir = env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/home/pi"));

        let mut systems = HashMap::new();
        systems.insert(
            "nes".to_string(),
            SystemConfig {
                default_core: "nestopia_libretro.so".to_string(),
                accepted_extensions: vec!["nes".to_string()],
                extra_args: vec![],
            },
        );
        systems.insert(
            "snes".to_string(),
            SystemConfig {
                default_core: "snes9x_libretro.so".to_string(),
                accepted_extensions: vec!["sfc".to_string(), "smc".to_string()],
                extra_args: vec![],
            },
        );
        systems.insert(
            "gba".to_string(),
            SystemConfig {
                default_core: "mgba_libretro.so".to_string(),
                accepted_extensions: vec!["gba".to_string()],
                extra_args: vec![],
            },
        );

        Self {
            retroarch: RetroArchConfig {
                binary_path: PathBuf::from("retroarch"),
                cores_dir: PathBuf::from("/usr/lib/libretro"),
                extra_args: vec!["--verbose".to_string()],
            },
            library: LibraryConfig {
                roms_dir: home_dir.join("pi/ROMs"),
                bios_dir: home_dir.join("pi/BIOS"),
            },
            systems,
            history: HistoryConfig::default(),
        }
    }

    pub fn resolve_system_key_for_extension(&self, extension: &str) -> Option<String> {
        let normalized = extension.to_ascii_lowercase();
        self.systems.iter().find_map(|(system_key, system_config)| {
            let has_match = system_config
                .accepted_extensions
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(&normalized));
            if has_match {
                Some(system_key.clone())
            } else {
                None
            }
        })
    }

    fn migrate_legacy_paths_if_needed(&mut self, config_path: &Path) -> anyhow::Result<()> {
        let legacy_roms_path = Path::new("/home/pi/ROMs");
        if self.library.roms_dir != legacy_roms_path {
            return Ok(());
        }

        let Some(home_dir) = env::var_os("HOME").map(PathBuf::from) else {
            return Ok(());
        };

        let candidates = [home_dir.join("pi/ROMs"), home_dir.join("ROMs")];
        let discovered = candidates.into_iter().find(|path| path.exists());

        if let Some(new_path) = discovered {
            self.library.roms_dir = new_path;
            self.save_to_file(config_path)?;
        }

        Ok(())
    }

    fn migrate_retroarch_defaults_if_needed(&mut self, config_path: &Path) -> anyhow::Result<()> {
        let mut changed = false;
        let configured_binary = self.retroarch.binary_path.clone();

        if configured_binary.is_absolute() && !configured_binary.exists() {
            if let Some(found) = find_executable_in_path("retroarch") {
                self.retroarch.binary_path = found;
            } else {
                self.retroarch.binary_path = PathBuf::from("retroarch");
            }
            changed = true;
        }

        let configured_cores_dir = self.retroarch.cores_dir.clone();
        if !configured_cores_dir.exists() {
            if let Some(found_dir) = discover_libretro_dir() {
                self.retroarch.cores_dir = found_dir;
                changed = true;
            }
        }

        if changed {
            self.save_to_file(config_path)?;
        }

        Ok(())
    }
}

fn find_executable_in_path(program_name: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for entry in env::split_paths(&path_var) {
        let candidate = entry.join(program_name);
        if candidate.exists() && candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn discover_libretro_dir() -> Option<PathBuf> {
    let fixed_candidates = [
        PathBuf::from("/usr/lib/libretro"),
        PathBuf::from("/usr/local/lib/libretro"),
        PathBuf::from("/var/lib/snapd/hostfs/usr/lib/libretro"),
    ];

    for candidate in fixed_candidates {
        if candidate.exists() {
            return Some(candidate);
        }
    }

    let usr_lib = Path::new("/usr/lib");
    let Ok(entries) = fs::read_dir(usr_lib) else {
        return None;
    };

    for item in entries.flatten() {
        let path = item.path().join("libretro");
        if path.exists() {
            return Some(path);
        }
    }

    None
}
