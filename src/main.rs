mod catalog;
mod config;
mod launcher;
mod theme;

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use catalog::{Catalog, GameEntry};
use config::{LauncherConfig, SystemConfig};
use directories::ProjectDirs;
use eframe::egui::{self, Color32, Frame, Margin, RichText, Rounding, Stroke};

fn main() -> anyhow::Result<()> {
    migrate_legacy_data_if_needed()?;

    let config_path = resolve_config_path()?;
    let catalog_path = resolve_catalog_path()?;
    let config = LauncherConfig::load_or_create(&config_path)?;
    let _ = config.ensure_system_library_dirs();
    let mut catalog = Catalog::open(&catalog_path)?;
    let scanned_count = catalog.sync_with_filesystem(&config).unwrap_or(0);
    let metadata_updated = catalog.refresh_metadata_cache(&config).unwrap_or(0);
    let games = catalog.list_games().unwrap_or_default();
    let favorites = catalog.list_favorites().unwrap_or_default();
    let recent = catalog.list_recent(10).unwrap_or_default();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("RPi Open Emulator")
            .with_app_id("dev.malack.rpi_open_emulator")
            .with_min_inner_size([720.0, 520.0]),
        ..Default::default()
    };

    eframe::run_native(
        "RPi Open Emulator",
        options,
        Box::new(move |cc| {
            theme::apply(&cc.egui_ctx);
            let (game_done_tx, game_done_rx) = mpsc::channel();
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
                game_done_tx,
                game_done_rx,
                game_session_busy: false,
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
    game_done_tx: Sender<launcher::RetroArchSessionResult>,
    game_done_rx: Receiver<launcher::RetroArchSessionResult>,
    /// RetroArch está em execução (thread de espera ativa).
    game_session_busy: bool,
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
        self.poll_retroarch_completion();

        let mut panel_frame = Frame::central_panel(&ctx.style());
        panel_frame.fill = theme::BG_DEEP;
        panel_frame.inner_margin = Margin::same(14.0);

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                self.render_brand_header(ui);
                ui.add_space(10.0);
                self.render_control_bar(ui);
                ui.add_space(8.0);
                self.render_status_line(ui);
                ui.add_space(12.0);

                if self.active_view == AppView::Settings {
                    Frame::none()
                        .fill(theme::BG_CARD)
                        .rounding(Rounding::same(10.0))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(60, 72, 105)))
                        .inner_margin(Margin::same(16.0))
                        .show(ui, |ui| {
                            ui.set_min_height(ui.available_height());
                            self.render_settings_panel(ui);
                        });
                    return;
                }

                theme::section_heading(ui, "Recentes", theme::ACCENT);
                if self.recent.is_empty() {
                    ui.label(
                        RichText::new("Nenhum jogo recente — abra um titulo na Biblioteca.")
                            .color(theme::TEXT_DIM)
                            .italics(),
                    );
                } else {
                    egui::ScrollArea::vertical()
                        .id_source("recent_scroll")
                        .max_height(150.0)
                        .show(ui, |ui| {
                            for game in self.recent.clone() {
                                self.render_game_row(ui, &game);
                            }
                        });
                }

                theme::section_heading(ui, "Favoritos", theme::WARM);
                if self.favorites.is_empty() {
                    ui.label(
                        RichText::new("Nenhum favorito — use a estrela na lista de jogos.")
                            .color(theme::TEXT_DIM)
                            .italics(),
                    );
                } else {
                    egui::ScrollArea::vertical()
                        .id_source("favorites_scroll")
                        .max_height(170.0)
                        .show(ui, |ui| {
                            for game in self.favorites.clone() {
                                self.render_game_row(ui, &game);
                            }
                        });
                }

                theme::section_heading(ui, "Biblioteca", theme::ACTION);
                if self.games.is_empty() {
                    ui.label(
                        RichText::new("Nenhuma ROM compativel — verifique a pasta em Configuracoes.")
                            .color(theme::TEXT_DIM)
                            .italics(),
                    );
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
    fn render_brand_header(&self, ui: &mut egui::Ui) {
        Frame::none()
            .fill(theme::BG_PANEL)
            .rounding(Rounding::same(10.0))
            .stroke(Stroke::new(1.0, Color32::from_rgb(55, 65, 95)))
            .inner_margin(Margin::symmetric(16.0, 14.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    Frame::none()
                        .fill(theme::ACCENT)
                        .rounding(Rounding::same(6.0))
                        .inner_margin(Margin::symmetric(10.0, 6.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("PWR")
                                    .monospace()
                                    .strong()
                                    .size(13.0)
                                    .color(Color32::BLACK),
                            );
                        });
                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("RPI Open Emulator")
                                .strong()
                                .size(22.0)
                                .color(theme::TEXT_MAIN),
                        );
                        ui.label(
                            RichText::new("Biblioteca local + RetroArch — estilo console")
                                .size(13.0)
                                .color(theme::TEXT_DIM),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        Frame::none()
                            .fill(theme::WARM)
                            .rounding(Rounding::same(20.0))
                            .inner_margin(Margin::symmetric(12.0, 6.0))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(format!("{} jogos", self.games.len()))
                                        .strong()
                                        .color(Color32::BLACK),
                                );
                            });
                    });
                });
                ui.add_space(6.0);
                ui.label(
                    RichText::new(self.config_path.display().to_string())
                        .small()
                        .color(theme::TEXT_DIM),
                );
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("RetroArch:")
                            .small()
                            .color(theme::TEXT_DIM),
                    );
                    ui.label(
                        RichText::new(self.config.retroarch.binary_path.display().to_string())
                            .small()
                            .color(theme::ACCENT),
                    );
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new("ROMs:")
                            .small()
                            .color(theme::TEXT_DIM),
                    );
                    ui.label(
                        RichText::new(self.config.library.roms_dir.display().to_string())
                            .small()
                            .color(theme::TEXT_MAIN),
                    );
                });
            });
    }

    fn render_control_bar(&mut self, ui: &mut egui::Ui) {
        Frame::none()
            .fill(theme::BG_CARD)
            .rounding(Rounding::same(10.0))
            .stroke(Stroke::new(1.0, Color32::from_rgb(50, 58, 82)))
            .inner_margin(Margin::symmetric(12.0, 10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if theme::outline_button(ui, "Sincronizar biblioteca", theme::ACCENT).clicked() {
                        self.sync_library();
                    }
                    if theme::outline_button(ui, "Atualizar metadata", theme::WARM).clicked() {
                        self.refresh_metadata();
                    }
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);
                    let lib_sel = self.active_view == AppView::Library;
                    if ui
                        .selectable_label(
                            lib_sel,
                            RichText::new("Biblioteca").color(if lib_sel {
                                theme::TEXT_MAIN
                            } else {
                                theme::TEXT_DIM
                            }),
                        )
                        .clicked()
                    {
                        self.active_view = AppView::Library;
                    }
                    let set_sel = self.active_view == AppView::Settings;
                    if ui
                        .selectable_label(
                            set_sel,
                            RichText::new("Configuracoes").color(if set_sel {
                                theme::TEXT_MAIN
                            } else {
                                theme::TEXT_DIM
                            }),
                        )
                        .clicked()
                    {
                        self.active_view = AppView::Settings;
                        self.settings_draft = SettingsDraft::from_config(&self.config);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(last_game) = self.config.history.last_game_path.as_ref() {
                            ui.label(
                                RichText::new(format!("Ultimo: {}", display_file_name(last_game)))
                                    .small()
                                    .color(theme::ACCENT),
                            );
                        } else {
                            ui.label(
                                RichText::new("Ultimo: —")
                                    .small()
                                    .color(theme::TEXT_DIM),
                            );
                        }
                    });
                });
            });
    }

    fn render_status_line(&self, ui: &mut egui::Ui) {
        let msg_color =
            if self.status.contains("Falha") || self.status.contains("Erro") {
                Color32::from_rgb(255, 130, 150)
            } else {
                theme::TEXT_MAIN
            };
        ui.horizontal(|ui| {
            ui.label(RichText::new("Status").strong().color(theme::ACCENT));
            ui.label(RichText::new(" — ").color(theme::TEXT_DIM));
            ui.label(RichText::new(&self.status).color(msg_color));
        });
    }

    fn render_settings_panel(&mut self, ui: &mut egui::Ui) {
        const FOOTER_H: f32 = 56.0;

        ui.vertical(|ui| {
            let scroll_h = (ui.available_height() - FOOTER_H).max(160.0);
            egui::ScrollArea::vertical()
                .id_source("settings_main_scroll")
                .max_height(scroll_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    self.render_settings_scroll_body(ui);
                });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if theme::outline_button(ui, "Recarregar do arquivo", theme::ACCENT).clicked() {
                    self.settings_draft = SettingsDraft::from_config(&self.config);
                    self.status = "Configuracoes recarregadas do arquivo".to_string();
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if theme::action_button(ui, "  Salvar configuracoes  ").clicked() {
                        self.save_settings_from_draft();
                    }
                });
            });
        });
    }

    /// Conteudo rolavel das configuracoes (caminhos, sistemas, novo sistema).
    fn render_settings_scroll_body(&mut self, ui: &mut egui::Ui) {
        theme::section_heading(ui, "Configuracoes", theme::ACTION);
        ui.label(
            RichText::new("Edite e salve sem alterar o TOML manualmente.")
                .color(theme::TEXT_DIM),
        );
        ui.label(
            RichText::new(
                "ROMs e BIOS por sistema: dentro das pastas base, use uma subpasta com o nome de cada sistema (ex.: nes, snes, gba).",
            )
            .small()
            .color(theme::TEXT_DIM),
        );
        ui.add_space(6.0);

        egui::Grid::new("settings_paths_grid")
            .num_columns(2)
            .spacing([14.0, 10.0])
            .min_col_width(160.0)
            .show(ui, |ui| {
                ui.label(
                    RichText::new("RetroArch - Binario")
                        .color(theme::TEXT_DIM),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.settings_draft.retroarch_binary)
                        .desired_width(ui.available_width()),
                );
                ui.end_row();

                ui.label(
                    RichText::new("RetroArch - Pasta de cores")
                        .color(theme::TEXT_DIM),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.settings_draft.cores_dir)
                        .desired_width(ui.available_width()),
                );
                ui.end_row();

                ui.label(
                    RichText::new("Biblioteca - Pasta ROMs")
                        .color(theme::TEXT_DIM),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.settings_draft.roms_dir)
                        .desired_width(ui.available_width()),
                );
                ui.end_row();

                ui.label(
                    RichText::new("Biblioteca - Pasta BIOS")
                        .color(theme::TEXT_DIM),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.settings_draft.bios_dir)
                        .desired_width(ui.available_width()),
                );
                ui.end_row();
            });

        ui.add_space(8.0);
        ui.separator();
        theme::section_heading(ui, "Sistemas configurados", theme::ACCENT);

        let mut remove_index: Option<usize> = None;
        for (index, system) in self.settings_draft.systems.iter_mut().enumerate() {
            let accent = theme::accent_for_system(&system.key);
            Frame::none()
                .fill(theme::BG_PANEL)
                .rounding(Rounding::same(8.0))
                .stroke(Stroke::new(1.0, accent.linear_multiply(0.4)))
                .inner_margin(Margin::symmetric(12.0, 10.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&system.key)
                                .strong()
                                .size(15.0)
                                .color(accent),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if theme::outline_button(
                                ui,
                                "Remover",
                                Color32::from_rgb(255, 120, 130),
                            )
                            .clicked()
                            {
                                remove_index = Some(index);
                            }
                        });
                    });
                    ui.add_space(6.0);
                    egui::Grid::new(egui::Id::new("sys_edit").with(index))
                        .num_columns(2)
                        .spacing([10.0, 6.0])
                        .min_col_width(120.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new("Chave").small().color(theme::TEXT_DIM));
                            ui.add(
                                egui::TextEdit::singleline(&mut system.key)
                                    .desired_width(ui.available_width()),
                            );
                            ui.end_row();
                            ui.label(RichText::new("Core padrao").small().color(theme::TEXT_DIM));
                            ui.add(
                                egui::TextEdit::singleline(&mut system.default_core)
                                    .desired_width(ui.available_width()),
                            );
                            ui.end_row();
                            ui.label(
                                RichText::new("Extensoes (csv)")
                                    .small()
                                    .color(theme::TEXT_DIM),
                            );
                            ui.add(
                                egui::TextEdit::singleline(
                                    &mut system.accepted_extensions_csv,
                                )
                                .desired_width(ui.available_width()),
                            );
                            ui.end_row();
                            ui.label(
                                RichText::new("Args extras (csv)")
                                    .small()
                                    .color(theme::TEXT_DIM),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut system.extra_args_csv)
                                    .desired_width(ui.available_width()),
                            );
                            ui.end_row();
                        });
                });
            ui.add_space(10.0);
        }

        if let Some(index) = remove_index {
            if index < self.settings_draft.systems.len() {
                let removed_key = self.settings_draft.systems[index].key.clone();
                self.settings_draft.systems.remove(index);
                self.status = format!("Sistema removido do rascunho: {}", removed_key);
            }
        }

        ui.separator();
        theme::section_heading(ui, "Adicionar sistema", theme::WARM);
        egui::Grid::new("settings_new_system_grid")
            .num_columns(2)
            .spacing([14.0, 10.0])
            .min_col_width(160.0)
            .show(ui, |ui| {
                ui.label(RichText::new("Chave").color(theme::TEXT_DIM));
                ui.add(
                    egui::TextEdit::singleline(&mut self.settings_draft.new_system_key)
                        .desired_width(ui.available_width()),
                );
                ui.end_row();

                ui.label(RichText::new("Core padrao").color(theme::TEXT_DIM));
                ui.add(
                    egui::TextEdit::singleline(&mut self.settings_draft.new_system_core)
                        .desired_width(ui.available_width()),
                );
                ui.end_row();

                ui.label(RichText::new("Extensoes (csv)").color(theme::TEXT_DIM));
                ui.add(
                    egui::TextEdit::singleline(
                        &mut self.settings_draft.new_system_extensions_csv,
                    )
                    .desired_width(ui.available_width()),
                );
                ui.end_row();

                ui.label(RichText::new("Args extras (csv)").color(theme::TEXT_DIM));
                ui.add(
                    egui::TextEdit::singleline(&mut self.settings_draft.new_system_args_csv)
                        .desired_width(ui.available_width()),
                );
                ui.end_row();
            });

        ui.add_space(8.0);
        if theme::outline_button(ui, "Adicionar sistema ao rascunho", theme::ACCENT).clicked() {
            self.add_system_to_draft();
        }
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

        if let Err(err) = self.config.ensure_system_library_dirs() {
            self.status = format!(
                "Configuracoes salvas, mas falhou ao criar pastas por sistema: {}",
                err
            );
            self.sync_library();
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

    fn poll_retroarch_completion(&mut self) {
        while let Ok(session) = self.game_done_rx.try_recv() {
            self.game_session_busy = false;
            let file_label = session
                .rom_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("ROM")
                .to_string();

            match &session.result {
                Ok(status) if status.success() => {
                    self.config.history.last_game_path = Some(session.rom_path.clone());
                    if let Err(err) = self.config.save_to_file(&self.config_path) {
                        self.status = format!(
                            "Jogo encerrado, mas nao foi possivel salvar historico: {}",
                            err
                        );
                        continue;
                    }
                    if let Err(err) = self.catalog.mark_played(&session.rom_path) {
                        self.status = format!(
                            "Jogo encerrado, mas falhou ao registrar historico: {}",
                            err
                        );
                        continue;
                    }
                    if let Err(err) = self.reload_lists() {
                        self.status =
                            format!("Jogo encerrado, mas falhou ao atualizar lista: {}", err);
                        continue;
                    }
                    self.status = format!("Jogo encerrado: {}", file_label);
                }
                Ok(status) => {
                    self.status = format!(
                        "RetroArch saiu com codigo {:?} ({})",
                        status.code(),
                        file_label
                    );
                }
                Err(err) => {
                    self.status = format!("Falha ao executar jogo ({file_label}): {err:#}");
                }
            }
        }
    }

    fn launch_game(&mut self, ctx: &egui::Context, game: &GameEntry) {
        if self.game_session_busy {
            self.status =
                "Ja existe uma sessao RetroArch em andamento — feche o jogo antes de abrir outro."
                    .to_string();
            return;
        }

        self.game_session_busy = true;
        self.status = format!("Iniciando {}…", game.file_name);

        let config = self.config.clone();
        let rom_path = game.path.clone();
        let tx = self.game_done_tx.clone();
        let ctx = ctx.clone();

        std::thread::spawn(move || {
            let result = launcher::run_retroarch_blocking(&config, &rom_path);
            let _ = tx.send(launcher::RetroArchSessionResult { rom_path, result });
            ctx.request_repaint();
        });
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
        let accent = theme::accent_for_system(&game.system_key);
        Frame::none()
            .fill(theme::BG_CARD)
            .rounding(Rounding::same(10.0))
            .stroke(Stroke::new(1.0, accent.linear_multiply(0.35)))
            .inner_margin(Margin::symmetric(12.0, 10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let favorite_mark = if game.is_favorite { "★" } else { "☆" };
                    ui.label(
                        RichText::new(favorite_mark)
                            .size(18.0)
                            .color(if game.is_favorite {
                                theme::WARM
                            } else {
                                theme::TEXT_DIM
                            }),
                    );
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(&game.title)
                                .strong()
                                .size(16.0)
                                .color(theme::TEXT_MAIN),
                        );
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!(".{}", game.extension))
                                    .small()
                                    .color(theme::TEXT_DIM),
                            );
                            ui.label(RichText::new(" | ").small().color(theme::TEXT_DIM));
                            ui.label(
                                RichText::new(&game.system_key)
                                    .small()
                                    .strong()
                                    .color(accent),
                            );
                            ui.label(RichText::new(" | ").small().color(theme::TEXT_DIM));
                            ui.label(
                                RichText::new(format!("{}x", game.play_count))
                                    .small()
                                    .color(theme::TEXT_DIM),
                            );
                        });
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if game.is_favorite {
                            if theme::outline_button(ui, "Desfavoritar", theme::WARM).clicked() {
                                self.set_favorite(game, false);
                            }
                        } else if theme::outline_button(ui, "Favoritar", theme::WARM).clicked() {
                            self.set_favorite(game, true);
                        }
                        ui.add_space(6.0);
                        if theme::action_button(ui, "  Jogar  ").clicked() {
                            self.launch_game(ui.ctx(), game);
                        }
                    });
                });
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!(
                        "Arquivo: {} | Capa: {} | {} | {}",
                        game.file_name,
                        game.cover_path
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "sem capa".to_string()),
                        game.metadata_source,
                        format_last_played_relative(game.last_played_at)
                    ))
                    .small()
                    .color(theme::TEXT_DIM),
                );
                if let Some(description) = game.description.as_ref() {
                    ui.label(
                        RichText::new(format!("Descricao: {}", description))
                            .small()
                            .color(theme::TEXT_DIM),
                    );
                }
            });
        ui.add_space(6.0);
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
