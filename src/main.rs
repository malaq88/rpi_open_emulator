mod config;
mod catalog;
mod launcher;

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use catalog::{Catalog, GameEntry};
use config::{LauncherConfig, SystemConfig};
use directories::ProjectDirs;
use eframe::egui;

fn main() -> anyhow::Result<()> {
    migrate_legacy_data_if_needed()?;

    let config_path = resolve_config_path()?;
    let catalog_path = resolve_catalog_path()?;
    let config = LauncherConfig::load_or_create(&config_path)?;
    let mut catalog = Catalog::open(&catalog_path)?;
    let scanned_count = catalog.sync_with_filesystem(&config).unwrap_or(0);
    let metadata_updated = catalog.refresh_metadata_cache(&config).unwrap_or(0);
    let games = catalog.list_games().unwrap_or_default();
    let favorites = catalog.list_favorites().unwrap_or_default();
    let recent = catalog.list_recent(10).unwrap_or_default();

    let options = eframe::NativeOptions {
        ..Default::default()
    };

    eframe::run_native(
        "RPI Open Emulator",
        options,
        Box::new(move |_cc| {
            let settings_draft = SettingsDraft::from_config(&config);
            let initial_status = if config.library.roms_dir.exists() {
                format!(
                    "{} ROM(s) no catalogo ({} atualizada(s) no scan, {} com metadata)",
                    games.len(),
                    scanned_count,
                    metadata_updated
                )
            } else {
                format!(
                    "Pasta de ROMs nao encontrada: {}",
                    config.library.roms_dir.display()
                )
            };
            Box::new(LauncherApp {
                config_path: config_path.clone(),
                config,
                catalog,
                games,
                favorites,
                recent,
                status: initial_status,
                active_view: AppView::Library,
                settings_draft,
            })
        }),
    )
    .map_err(|err| anyhow::anyhow!(err.to_string()))
}

fn resolve_config_path() -> anyhow::Result<PathBuf> {
    let project_dirs = ProjectDirs::from("dev", "malack", "rpi_open_emulator")
        .context("nao foi possivel resolver diretorio de configuracao")?;
    Ok(project_dirs.config_dir().join("config.toml"))
}

fn resolve_catalog_path() -> anyhow::Result<PathBuf> {
    let project_dirs = ProjectDirs::from("dev", "malack", "rpi_open_emulator")
        .context("nao foi possivel resolver diretorio de dados")?;
    Ok(project_dirs.data_local_dir().join("catalog.sqlite3"))
}

/// Copia dados da versao anterior (`rpi5-launcher`) se o novo diretorio ainda nao existir.
fn migrate_legacy_data_if_needed() -> anyhow::Result<()> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Ok(());
    };

    let legacy_config = home.join(".config/rpi5-launcher/config.toml");
    let legacy_catalog = home.join(".local/share/rpi5-launcher/catalog.sqlite3");

    let new_dirs = ProjectDirs::from("dev", "malack", "rpi_open_emulator")
        .context("nao foi possivel resolver ProjectDirs para migracao")?;
    let new_config = new_dirs.config_dir().join("config.toml");
    let new_catalog = new_dirs.data_local_dir().join("catalog.sqlite3");

    if !new_config.exists() && legacy_config.exists() {
        fs::create_dir_all(new_dirs.config_dir()).context("criar diretorio de configuracao")?;
        fs::copy(&legacy_config, &new_config).context("copiar configuracao legada rpi5-launcher")?;
    }

    if !new_catalog.exists() && legacy_catalog.exists() {
        fs::create_dir_all(new_dirs.data_local_dir()).context("criar diretorio de dados")?;
        fs::copy(&legacy_catalog, &new_catalog).context("copiar catalogo legado rpi5-launcher")?;
    }

    Ok(())
}

struct LauncherApp {
    config_path: PathBuf,
    config: LauncherConfig,
    catalog: Catalog,
    games: Vec<GameEntry>,
    favorites: Vec<GameEntry>,
    recent: Vec<GameEntry>,
    status: String,
    active_view: AppView,
    settings_draft: SettingsDraft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppView {
    Library,
    Settings,
}

#[derive(Debug, Clone)]
struct SettingsDraft {
    retroarch_binary: String,
    cores_dir: String,
    roms_dir: String,
    bios_dir: String,
    systems: Vec<SystemDraft>,
    new_system_key: String,
    new_system_core: String,
    new_system_extensions_csv: String,
    new_system_args_csv: String,
}

#[derive(Debug, Clone)]
struct SystemDraft {
    key: String,
    default_core: String,
    accepted_extensions_csv: String,
    extra_args_csv: String,
}

impl SettingsDraft {
    fn from_config(config: &LauncherConfig) -> Self {
        let mut systems = config
            .systems
            .iter()
            .map(|(key, system)| SystemDraft {
                key: key.clone(),
                default_core: system.default_core.clone(),
                accepted_extensions_csv: system.accepted_extensions.join(", "),
                extra_args_csv: system.extra_args.join(", "),
            })
            .collect::<Vec<_>>();
        systems.sort_by(|a, b| a.key.cmp(&b.key));

        Self {
            retroarch_binary: config.retroarch.binary_path.display().to_string(),
            cores_dir: config.retroarch.cores_dir.display().to_string(),
            roms_dir: config.library.roms_dir.display().to_string(),
            bios_dir: config.library.bios_dir.display().to_string(),
            systems,
            new_system_key: String::new(),
            new_system_core: String::new(),
            new_system_extensions_csv: String::new(),
            new_system_args_csv: String::new(),
        }
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("RPI Open Emulator");
            ui.label(format!(
                "Arquivo de configuracao: {}",
                self.config_path.display()
            ));
            ui.separator();
            ui.label(format!(
                "RetroArch: {}",
                self.config.retroarch.binary_path.display()
            ));
            ui.label(format!("Pasta ROMs: {}", self.config.library.roms_dir.display()));
            ui.label(format!("Cores cadastrados: {}", self.config.systems.len()));
            ui.label("Stack UI escolhida: egui + eframe");
            ui.label(format!("Status: {}", self.status));

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Sincronizar biblioteca").clicked() {
                    self.sync_library();
                }
                if ui.button("Atualizar metadata offline").clicked() {
                    self.refresh_metadata();
                }
                ui.separator();
                if ui
                    .selectable_label(self.active_view == AppView::Library, "Biblioteca")
                    .clicked()
                {
                    self.active_view = AppView::Library;
                }
                if ui
                    .selectable_label(self.active_view == AppView::Settings, "Configuracoes")
                    .clicked()
                {
                    self.active_view = AppView::Settings;
                    self.settings_draft = SettingsDraft::from_config(&self.config);
                }

                if let Some(last_game) = self.config.history.last_game_path.as_ref() {
                    ui.label(format!("Ultimo jogo: {}", display_file_name(last_game)));
                } else {
                    ui.label("Ultimo jogo: nenhum");
                }
            });

            ui.separator();
            if self.active_view == AppView::Settings {
                self.render_settings_panel(ui);
                return;
            }

            ui.heading("Recentes");
            if self.recent.is_empty() {
                ui.label("Nenhum jogo recente.");
            } else {
                egui::ScrollArea::vertical()
                    .id_source("recent_scroll")
                    .max_height(140.0)
                    .show(ui, |ui| {
                        for game in self.recent.clone() {
                            self.render_game_row(ui, &game);
                        }
                    });
            }

            ui.separator();
            ui.heading("Favoritos");
            if self.favorites.is_empty() {
                ui.label("Nenhum favorito.");
            } else {
                egui::ScrollArea::vertical()
                    .id_source("favorites_scroll")
                    .max_height(160.0)
                    .show(ui, |ui| {
                        for game in self.favorites.clone() {
                            self.render_game_row(ui, &game);
                        }
                    });
            }

            ui.separator();
            ui.heading("Biblioteca");
            if self.games.is_empty() {
                ui.label("Nenhuma ROM compativel catalogada.");
                return;
            }
            egui::ScrollArea::vertical()
                .id_source("library_scroll")
                .show(ui, |ui| {
                    for game in self.games.clone() {
                        self.render_game_row(ui, &game);
                    }
                });
        });
    }
}

impl LauncherApp {
    fn render_settings_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Configuracoes");
        ui.label("Edite e salve sem precisar alterar TOML manualmente.");

        ui.label("RetroArch - Binario");
        ui.text_edit_singleline(&mut self.settings_draft.retroarch_binary);
        ui.label("RetroArch - Pasta de cores");
        ui.text_edit_singleline(&mut self.settings_draft.cores_dir);
        ui.label("Biblioteca - Pasta ROMs");
        ui.text_edit_singleline(&mut self.settings_draft.roms_dir);
        ui.label("Biblioteca - Pasta BIOS");
        ui.text_edit_singleline(&mut self.settings_draft.bios_dir);

        ui.separator();
        ui.label("Sistemas");
        let mut remove_index: Option<usize> = None;
        egui::ScrollArea::vertical()
            .id_source("settings_systems_scroll")
            .max_height(300.0)
            .show(ui, |ui| {
                for (index, system) in self.settings_draft.systems.iter_mut().enumerate() {
                    ui.group(|ui| {
                        ui.label(format!("Sistema: {}", system.key));
                        ui.horizontal(|ui| {
                            if ui.button("Remover sistema").clicked() {
                                remove_index = Some(index);
                            }
                        });
                        ui.label("Chave");
                        ui.text_edit_singleline(&mut system.key);
                        ui.label("Core padrao");
                        ui.text_edit_singleline(&mut system.default_core);
                        ui.label("Extensoes (csv)");
                        ui.text_edit_singleline(&mut system.accepted_extensions_csv);
                        ui.label("Args extras (csv)");
                        ui.text_edit_singleline(&mut system.extra_args_csv);
                    });
                    ui.add_space(8.0);
                }
            });
        if let Some(index) = remove_index {
            if index < self.settings_draft.systems.len() {
                let removed_key = self.settings_draft.systems[index].key.clone();
                self.settings_draft.systems.remove(index);
                self.status = format!("Sistema removido do rascunho: {}", removed_key);
            }
        }

        ui.separator();
        ui.label("Adicionar sistema");
        ui.label("Chave");
        ui.text_edit_singleline(&mut self.settings_draft.new_system_key);
        ui.label("Core padrao");
        ui.text_edit_singleline(&mut self.settings_draft.new_system_core);
        ui.label("Extensoes (csv)");
        ui.text_edit_singleline(&mut self.settings_draft.new_system_extensions_csv);
        ui.label("Args extras (csv)");
        ui.text_edit_singleline(&mut self.settings_draft.new_system_args_csv);
        if ui.button("Adicionar sistema ao rascunho").clicked() {
            self.add_system_to_draft();
        }

        ui.horizontal(|ui| {
            if ui.button("Recarregar do arquivo").clicked() {
                self.settings_draft = SettingsDraft::from_config(&self.config);
                self.status = "Configuracoes recarregadas do arquivo".to_string();
            }
            if ui.button("Salvar configuracoes").clicked() {
                self.save_settings_from_draft();
            }
        });
    }

    fn save_settings_from_draft(&mut self) {
        self.config.retroarch.binary_path = PathBuf::from(self.settings_draft.retroarch_binary.trim());
        self.config.retroarch.cores_dir = PathBuf::from(self.settings_draft.cores_dir.trim());
        self.config.library.roms_dir = PathBuf::from(self.settings_draft.roms_dir.trim());
        self.config.library.bios_dir = PathBuf::from(self.settings_draft.bios_dir.trim());

        let mut systems = std::collections::HashMap::new();
        let mut seen_keys = HashSet::new();
        for system in &self.settings_draft.systems {
            let normalized_key = system.key.trim().to_ascii_lowercase();
            if normalized_key.is_empty() {
                self.status = "Falha ao salvar: existe sistema com chave vazia".to_string();
                return;
            }
            if !seen_keys.insert(normalized_key.clone()) {
                self.status = format!("Falha ao salvar: chave duplicada '{}'", normalized_key);
                return;
            }

            let default_core = system.default_core.trim().to_string();
            if default_core.is_empty() {
                self.status = format!(
                    "Falha ao salvar: sistema '{}' com core padrao vazio",
                    normalized_key
                );
                return;
            }

            systems.insert(
                normalized_key,
                SystemConfig {
                    default_core,
                    accepted_extensions: parse_csv_values(&system.accepted_extensions_csv),
                    extra_args: parse_csv_values(&system.extra_args_csv),
                },
            );
        }
        self.config.systems = systems;

        if let Err(err) = self.config.save_to_file(&self.config_path) {
            self.status = format!("Falha ao salvar configuracoes: {}", err);
            return;
        }

        self.status = "Configuracoes salvas com sucesso".to_string();
        self.sync_library();
    }

    fn add_system_to_draft(&mut self) {
        let key = self.settings_draft.new_system_key.trim().to_ascii_lowercase();
        if key.is_empty() {
            self.status = "Informe a chave do novo sistema".to_string();
            return;
        }
        if self
            .settings_draft
            .systems
            .iter()
            .any(|system| system.key.eq_ignore_ascii_case(&key))
        {
            self.status = format!("Sistema '{}' ja existe no rascunho", key);
            return;
        }

        let default_core = self.settings_draft.new_system_core.trim().to_string();
        if default_core.is_empty() {
            self.status = "Informe o core padrao do novo sistema".to_string();
            return;
        }

        self.settings_draft.systems.push(SystemDraft {
            key: key.clone(),
            default_core,
            accepted_extensions_csv: self.settings_draft.new_system_extensions_csv.trim().to_string(),
            extra_args_csv: self.settings_draft.new_system_args_csv.trim().to_string(),
        });
        self.settings_draft
            .systems
            .sort_by(|a, b| a.key.cmp(&b.key));

        self.settings_draft.new_system_key.clear();
        self.settings_draft.new_system_core.clear();
        self.settings_draft.new_system_extensions_csv.clear();
        self.settings_draft.new_system_args_csv.clear();
        self.status = format!("Sistema adicionado ao rascunho: {}", key);
    }

    fn sync_library(&mut self) {
        if !self.config.library.roms_dir.exists() {
            self.status = format!(
                "Pasta de ROMs nao encontrada: {}",
                self.config.library.roms_dir.display()
            );
            return;
        }

        match self.catalog.sync_with_filesystem(&self.config) {
            Ok(scanned_count) => {
                let metadata_updated = self.catalog.refresh_metadata_cache(&self.config).unwrap_or(0);
                if let Err(err) = self.reload_lists() {
                    self.status = format!("Falha ao recarregar catalogo: {}", err);
                    return;
                }
                self.status = format!(
                    "{} ROM(s) no catalogo ({} atualizada(s) no scan, {} com metadata)",
                    self.games.len(),
                    scanned_count,
                    metadata_updated
                );
            }
            Err(err) => {
                self.status = format!("Erro ao sincronizar biblioteca: {}", err);
            }
        }
    }

    fn launch_game(&mut self, game: &GameEntry) {
        self.status = format!("Iniciando {}", game.file_name);

        match launcher::launch_game(&self.config, &game.path) {
            Ok(()) => {
                self.config.history.last_game_path = Some(game.path.clone());
                if let Err(err) = self.config.save_to_file(&self.config_path) {
                    self.status = format!(
                        "Jogo executado, mas nao foi possivel salvar historico: {}",
                        err
                    );
                    return;
                }
                if let Err(err) = self.catalog.mark_played(&game.path) {
                    self.status = format!("Jogo executado, mas falhou ao registrar historico: {}", err);
                    return;
                }
                if let Err(err) = self.reload_lists() {
                    self.status = format!("Jogo executado, mas falhou ao atualizar lista: {}", err);
                    return;
                }
                self.status = format!("Jogo encerrado: {}", game.file_name);
            }
            Err(err) => {
                self.status = format!("Falha ao executar jogo: {err:#}");
            }
        }
    }

    fn set_favorite(&mut self, game: &GameEntry, value: bool) {
        match self.catalog.set_favorite(&game.path, value) {
            Ok(()) => {
                if let Err(err) = self.reload_lists() {
                    self.status = format!("Favorito atualizado, mas falhou ao recarregar: {}", err);
                    return;
                }
                self.status = if value {
                    format!("Marcado como favorito: {}", game.file_name)
                } else {
                    format!("Removido dos favoritos: {}", game.file_name)
                };
            }
            Err(err) => {
                self.status = format!("Falha ao atualizar favorito: {}", err);
            }
        }
    }

    fn reload_lists(&mut self) -> anyhow::Result<()> {
        self.games = self.catalog.list_games()?;
        self.favorites = self.catalog.list_favorites()?;
        self.recent = self.catalog.list_recent(10)?;
        Ok(())
    }

    fn refresh_metadata(&mut self) {
        match self.catalog.refresh_metadata_cache(&self.config) {
            Ok(updated) => {
                if let Err(err) = self.reload_lists() {
                    self.status = format!("Metadata atualizada, mas falhou ao recarregar: {}", err);
                    return;
                }
                self.status = format!("Metadata offline atualizada para {} jogo(s)", updated);
            }
            Err(err) => {
                self.status = format!("Falha ao atualizar metadata offline: {}", err);
            }
        }
    }

    fn render_game_row(&mut self, ui: &mut egui::Ui, game: &GameEntry) {
        ui.horizontal(|ui| {
            let favorite_mark = if game.is_favorite { "★" } else { "☆" };
            ui.label(format!(
                "{} {} [.{} | {} | jogado {}x]",
                favorite_mark, game.title, game.extension, game.system_key, game.play_count
            ));

            if ui.button("Jogar").clicked() {
                self.launch_game(game);
            }

            if game.is_favorite {
                if ui.button("Desfavoritar").clicked() {
                    self.set_favorite(game, false);
                }
            } else if ui.button("Favoritar").clicked() {
                self.set_favorite(game, true);
            }
        });
        ui.small(format!(
            "Arquivo: {} | Capa: {} | Fonte metadata: {} | {}",
            game.file_name,
            game.cover_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "nao encontrada".to_string()),
            game.metadata_source,
            format_last_played_relative(game.last_played_at)
        ));
        if let Some(description) = game.description.as_ref() {
            ui.small(format!("Descricao: {}", description));
        }
        ui.separator();
    }
}

fn format_last_played_relative(ts: Option<i64>) -> String {
    let Some(ts) = ts else {
        return "ultimo: nunca".to_string();
    };
    let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs() as i64,
        Err(_) => return format!("ultimo: epoch {}", ts),
    };
    let delta = now.saturating_sub(ts);
    if delta < 60 {
        return "ultimo: agora ha pouco".to_string();
    }
    if delta < 3600 {
        return format!("ultimo: ha {} min", delta / 60);
    }
    if delta < 86400 {
        return format!("ultimo: ha {} h", delta / 3600);
    }
    format!("ultimo: ha {} dia(s)", delta / 86400)
}

fn parse_csv_values(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn display_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}
