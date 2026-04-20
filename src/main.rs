mod config;
mod core;
mod default_systems;
mod plugins;
mod product;
mod state;
mod theme;
mod themes;
mod ui;

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
};

use anyhow::Context;
use config::{LauncherConfig, SystemConfig};
use core::emulator::{RetroArchSessionResult, run_retroarch_blocking};
use core::library::{Catalog, GameEntry};
use core::scraper::ScraperPipeline;
use directories::ProjectDirs;
use eframe::egui::{self, Color32, Frame, Margin, RichText, Rounding, Stroke};
use plugins::PluginHost;
use product::FeatureFlags;
use state::filter_state::SidebarSection;
use state::{AppState, AppView};
use themes::ThemeId;
use ui::components::search_bar;
use ui::components::sidebar::{self, SidebarNavAction};
use ui::screens::game_detail::{self, DetailCommand};
use ui::screens::home::{self, HomeCommand};
use ui::screens::library::{self, LibraryCommand};

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
    let recent = catalog.list_recent(10).unwrap_or_default();
    let recently_added = catalog.list_recently_added(16).unwrap_or_default();
    let most_played = catalog.list_most_played(16).unwrap_or_default();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("RPi Open Emulator")
            .with_app_id("dev.malack.rpi_open_emulator")
            .with_min_inner_size([1000.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "RPi Open Emulator",
        options,
        Box::new(move |cc| {
            themes::apply(&cc.egui_ctx, ThemeId::DefaultDark);
            let (game_done_tx, game_done_rx) = mpsc::channel();
            let settings_draft = SettingsDraft::from_config(&config);
            let initial_status = if config.library.roms_dir.exists() {
                format!(
                    "{} ROM(s) no catálogo ({} atualizada(s) no scan, {} com metadata)",
                    games.len(),
                    scanned_count,
                    metadata_updated
                )
            } else {
                format!(
                    "Pasta de ROMs não encontrada: {}",
                    config.library.roms_dir.display()
                )
            };
            let mut plugin_host = PluginHost::default();
            plugin_host.register_scraper_id("offline-cache");
            let pipeline = ScraperPipeline::default();
            Box::new(LauncherApp {
                config_path: config_path.clone(),
                config,
                catalog,
                state: AppState::new(games, recent, recently_added, most_played),
                status: initial_status,
                settings_draft,
                game_done_tx,
                game_done_rx,
                game_session_busy: false,
                cover_texture_cache: HashMap::new(),
                ui_theme: ThemeId::DefaultDark,
                features: FeatureFlags::default(),
                plugin_host,
                scraper_pipeline: pipeline,
                settings_tab: SettingsTab::default(),
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
    state: AppState,
    status: String,
    settings_draft: SettingsDraft,
    game_done_tx: Sender<RetroArchSessionResult>,
    game_done_rx: Receiver<RetroArchSessionResult>,
    game_session_busy: bool,
    cover_texture_cache: HashMap<PathBuf, egui::TextureHandle>,
    ui_theme: ThemeId,
    features: FeatureFlags,
    plugin_host: PluginHost,
    scraper_pipeline: ScraperPipeline,
    settings_tab: SettingsTab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum SettingsTab {
    #[default]
    Interface,
    LibraryPaths,
    EmulatorCores,
    Controls,
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
        themes::apply(ctx, self.ui_theme);

        let mut panel_frame = Frame::central_panel(&ctx.style());
        panel_frame.fill = theme::BG_DEEP;
        panel_frame.inner_margin = Margin::same(12.0);

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                const APP_FOOTER_H: f32 = 44.0;

                ui.vertical(|ui| {
                    self.render_brand_header(ui);
                    ui.add_space(8.0);
                    self.render_status_line(ui);
                    ui.add_space(10.0);

                    let w = ui.available_width();
                    let mid_h = (ui.available_height() - APP_FOOTER_H).max(80.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(w, mid_h),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            let counts =
                                library::build_sidebar_counts(&self.state.games, &self.config);
                            let sidebar_w = 232.0;
                            let content_row_h = ui.available_height();
                            ui.horizontal(|ui| {
                                ui.set_min_height(content_row_h);

                                let mut nav = SidebarNavAction::None;
                                ui.allocate_ui_with_layout(
                                    egui::vec2(sidebar_w, content_row_h),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        nav = Frame::none()
                                            .fill(theme::BG_PANEL)
                                            .rounding(Rounding::same(12.0))
                                            .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
                                            .inner_margin(Margin::symmetric(10.0, 14.0))
                                            .show(ui, |ui| {
                                                ui.set_width(sidebar_w);
                                                ui.set_min_height(ui.available_height());
                                                sidebar::render_sidebar(
                                                    ui,
                                                    &mut self.state.filter,
                                                    &counts,
                                                    &self.state.selected_view,
                                                )
                                            })
                                            .inner;
                                    },
                                );

                                self.apply_sidebar_nav(nav);

                                ui.add_space(12.0);

                                ui.vertical(|ui| {
                                    self.render_global_toolbar(ui, ctx);

                                    match &self.state.selected_view {
                                        AppView::Dashboard => {
                                            let hcmds = home::render_home(
                                                ui,
                                                ctx,
                                                &self.state.games,
                                                &self.state.recent,
                                                &self.state.recently_added,
                                                &self.state.most_played,
                                                &self.config,
                                                &self.state.filter,
                                                &mut self.cover_texture_cache,
                                            );
                                            self.handle_home_commands(ctx, hcmds);
                                        }
                                        AppView::Library => {
                                            ui.set_min_height(ui.available_height());
                                            let lcmds = library::render_library_grid(
                                                ui,
                                                ctx,
                                                &self.state.games,
                                                &self.state.recent,
                                                &self.state.filter,
                                                &mut self.cover_texture_cache,
                                            );
                                            self.handle_library_commands(ctx, lcmds);
                                        }
                                        AppView::GameDetail(path) => {
                                            if let Some(game) = self
                                                .state
                                                .games
                                                .iter()
                                                .find(|g| g.path == *path)
                                                .cloned()
                                            {
                                                if let Some(dc) =
                                                    game_detail::render_game_detail(ui, &game)
                                                {
                                                    match dc {
                                                        DetailCommand::Back => {
                                                            self.state.selected_view =
                                                                AppView::Library;
                                                        }
                                                        DetailCommand::Launch(g) => {
                                                            self.launch_game(ctx, &g);
                                                        }
                                                        DetailCommand::Favorite(p, v) => {
                                                            if let Some(g) = self
                                                                .state
                                                                .games
                                                                .iter()
                                                                .find(|gr| gr.path == p)
                                                                .cloned()
                                                            {
                                                                self.set_favorite(&g, v);
                                                            }
                                                        }
                                                    }
                                                }
                                            } else {
                                                ui.label(
                                                    RichText::new(
                                                        "Jogo não encontrado no catálogo.",
                                                    )
                                                    .color(theme::TEXT_DIM),
                                                );
                                                if theme::outline_button(ui, "Voltar", theme::ACCENT)
                                                    .clicked()
                                                {
                                                    self.state.selected_view = AppView::Library;
                                                }
                                            }
                                        }
                                        AppView::Settings => {
                                            Frame::none()
                                                .fill(theme::BG_CARD)
                                                .rounding(Rounding::same(10.0))
                                                .stroke(Stroke::new(
                                                    1.0,
                                                    Color32::from_rgb(60, 72, 105),
                                                ))
                                                .inner_margin(Margin::same(16.0))
                                                .show(ui, |ui| {
                                                    ui.set_min_height(ui.available_height());
                                                    self.render_settings_panel(ui);
                                                });
                                        }
                                    }
                                });
                            });
                        },
                    );

                    ui.add_space(6.0);
                    self.render_app_footer(ui);
                });
            });
    }
}

impl LauncherApp {
    fn apply_sidebar_nav(&mut self, nav: SidebarNavAction) {
        match nav {
            SidebarNavAction::None => {}
            SidebarNavAction::GoHome => {
                self.state.selected_view = AppView::Dashboard;
            }
            SidebarNavAction::GoLibraryAll => {
                self.state.selected_view = AppView::Library;
                self.state.filter.section = SidebarSection::All;
            }
            SidebarNavAction::GoFavorites => {
                self.state.selected_view = AppView::Library;
                self.state.filter.section = SidebarSection::Favorites;
            }
            SidebarNavAction::GoRecent => {
                self.state.selected_view = AppView::Library;
                self.state.filter.section = SidebarSection::Recent;
            }
            SidebarNavAction::GoConsole(key) => {
                self.state.selected_view = AppView::Library;
                self.state.filter.section = SidebarSection::Console(key);
            }
            SidebarNavAction::GoSettings => {
                self.state.selected_view = AppView::Settings;
                self.settings_draft = SettingsDraft::from_config(&self.config);
            }
            SidebarNavAction::SyncLibrary => {
                self.sync_library();
            }
            SidebarNavAction::RefreshMetadata => {
                self.refresh_metadata();
            }
        }
    }

    fn render_global_toolbar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if !matches!(
            self.state.selected_view,
            AppView::Dashboard | AppView::Library
        ) {
            return;
        }

        Frame::none()
            .fill(theme::BG_CARD)
            .rounding(Rounding::same(10.0))
            .stroke(Stroke::new(1.0, Color32::from_rgb(50, 58, 82)))
            .inner_margin(Margin::symmetric(12.0, 10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let search_id = egui::Id::new("global_search_field");
                    if ctx.input(|i| i.key_pressed(egui::Key::F) && i.modifiers.ctrl) {
                        ctx.memory_mut(|m| m.request_focus(search_id));
                    }
                    let search_w = (ui.available_width() - 120.0).max(200.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(search_w, 40.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            search_bar::render_search_bar(
                                ui,
                                &mut self.state.filter.search_query,
                                search_id,
                            );
                        },
                    );
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        Frame::none()
                            .fill(theme::UI_CARD_ELEVATED)
                            .rounding(Rounding::same(8.0))
                            .inner_margin(Margin::symmetric(10.0, 6.0))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(format!("{}", self.state.games.len()))
                                        .strong()
                                        .color(theme::TEXT_MAIN),
                                );
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new("jogos")
                                        .small()
                                        .color(theme::TEXT_DIM),
                                );
                            });
                    });
                });
            });
        ui.add_space(10.0);
    }

    fn handle_home_commands(&mut self, ctx: &egui::Context, cmds: Vec<HomeCommand>) {
        for c in cmds {
            match c {
                HomeCommand::Launch(g) => self.launch_game(ctx, &g),
                HomeCommand::Favorite(p, v) => {
                    if let Some(g) = self.state.games.iter().find(|gr| gr.path == p).cloned() {
                        self.set_favorite(&g, v);
                    }
                }
                HomeCommand::OpenDetail(p) => {
                    self.state.selected_view = AppView::GameDetail(p);
                }
            }
        }
    }

    fn handle_library_commands(&mut self, ctx: &egui::Context, cmds: Vec<LibraryCommand>) {
        for c in cmds {
            match c {
                LibraryCommand::Launch(g) => self.launch_game(ctx, &g),
                LibraryCommand::Favorite(p, v) => {
                    if let Some(g) = self.state.games.iter().find(|gr| gr.path == p).cloned() {
                        self.set_favorite(&g, v);
                    }
                }
                LibraryCommand::OpenDetail(p) => {
                    self.state.selected_view = AppView::GameDetail(p);
                }
            }
        }
    }

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
                                RichText::new("HUB")
                                    .monospace()
                                    .strong()
                                    .size(13.0)
                                    .color(Color32::BLACK),
                            );
                        });
                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("RPi Open Emulator")
                                .strong()
                                .size(22.0)
                                .color(theme::TEXT_MAIN),
                        );
                        ui.label(
                            RichText::new("Plataforma de biblioteca retrô — UX estilo consola / Steam")
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
                                    RichText::new(format!("{} títulos", self.state.games.len()))
                                        .strong()
                                        .color(Color32::BLACK),
                                );
                            });
                    });
                });
                ui.add_space(4.0);
                ui.label(
                    RichText::new(self.config_path.display().to_string())
                        .small()
                        .color(theme::TEXT_DIM),
                );
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

    fn render_app_footer(&self, ui: &mut egui::Ui) {
        let scrape_line = format!(
            "Scraping: {} | {} jogo(s)",
            self.scraper_pipeline.describe(),
            self.state.games.len()
        );
        Frame::none()
            .fill(theme::BG_PANEL)
            .rounding(Rounding::same(8.0))
            .stroke(Stroke::new(1.0, Color32::from_rgb(55, 65, 95)))
            .inner_margin(Margin::symmetric(14.0, 10.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(scrape_line)
                            .size(12.0)
                            .color(theme::TEXT_DIM),
                    );
                });
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

    fn render_settings_scroll_body(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for (tab, label) in [
                (SettingsTab::Interface, "Interface"),
                (SettingsTab::LibraryPaths, "Diretórios"),
                (SettingsTab::EmulatorCores, "Núcleos"),
                (SettingsTab::Controls, "Controles"),
            ] {
                let sel = self.settings_tab == tab;
                if ui
                    .selectable_label(
                        sel,
                        RichText::new(label).color(if sel {
                            theme::TEXT_MAIN
                        } else {
                            theme::TEXT_DIM
                        }),
                    )
                    .clicked()
                {
                    self.settings_tab = tab;
                }
            }
        });
        ui.add_space(12.0);

        match self.settings_tab {
            SettingsTab::Interface => {
                theme::section_heading(ui, "Interface", theme::ACCENT);
                ui.label(
                    RichText::new("Tema visual (presets embutidos).")
                        .color(theme::TEXT_DIM),
                );
                ui.add_space(8.0);
                for t in ThemeId::ALL {
                    let allowed = self.features.theme_allowed(t);
                    let mut rt = RichText::new(t.label());
                    if !allowed {
                        rt = rt.italics().color(theme::TEXT_DIM);
                    }
                    if ui
                        .selectable_label(self.ui_theme == t && allowed, rt)
                        .clicked()
                        && allowed
                    {
                        self.ui_theme = t;
                    }
                }
                ui.add_space(14.0);
                ui.separator();
                theme::section_heading(ui, "Produto (roadmap)", theme::WARM);
                ui.label(
                    RichText::new(
                        "Gratuito: biblioteca local, metadados offline, temas base.\n\
                         Premium (futuro): scraping avançado, sync na nuvem, temas exclusivos.",
                    )
                    .color(theme::TEXT_DIM),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!(
                        "Plugins registados: {:?}",
                        self.plugin_host.list_scraper_ids()
                    ))
                    .small()
                    .color(theme::TEXT_DIM),
                );
            }
            SettingsTab::LibraryPaths => {
                theme::section_heading(ui, "Diretórios da biblioteca", theme::ACTION);
                ui.label(
                    RichText::new(
                        "ROMs e BIOS por sistema: subpastas com o nome de cada sistema (ex.: nes, snes).",
                    )
                    .small()
                    .color(theme::TEXT_DIM),
                );
                ui.add_space(8.0);
                egui::Grid::new("settings_paths_grid")
                    .num_columns(2)
                    .spacing([14.0, 10.0])
                    .min_col_width(160.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new("RetroArch — binário").color(theme::TEXT_DIM));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.settings_draft.retroarch_binary)
                                .desired_width(ui.available_width()),
                        );
                        ui.end_row();
                        ui.label(RichText::new("RetroArch — pasta de cores").color(theme::TEXT_DIM));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.settings_draft.cores_dir)
                                .desired_width(ui.available_width()),
                        );
                        ui.end_row();
                        ui.label(RichText::new("Biblioteca — pasta ROMs").color(theme::TEXT_DIM));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.settings_draft.roms_dir)
                                .desired_width(ui.available_width()),
                        );
                        ui.end_row();
                        ui.label(RichText::new("Biblioteca — pasta BIOS").color(theme::TEXT_DIM));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.settings_draft.bios_dir)
                                .desired_width(ui.available_width()),
                        );
                        ui.end_row();
                    });
            }
            SettingsTab::EmulatorCores => {
                theme::section_heading(ui, "Sistemas e núcleos", theme::ACCENT);
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
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if theme::outline_button(
                                            ui,
                                            "Remover",
                                            Color32::from_rgb(255, 120, 130),
                                        )
                                        .clicked()
                                        {
                                            remove_index = Some(index);
                                        }
                                    },
                                );
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
                                    ui.label(
                                        RichText::new("Core padrão").small().color(theme::TEXT_DIM),
                                    );
                                    ui.add(
                                        egui::TextEdit::singleline(&mut system.default_core)
                                            .desired_width(ui.available_width()),
                                    );
                                    ui.end_row();
                                    ui.label(
                                        RichText::new("Extensões (csv)")
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
                        ui.label(RichText::new("Core padrão").color(theme::TEXT_DIM));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.settings_draft.new_system_core)
                                .desired_width(ui.available_width()),
                        );
                        ui.end_row();
                        ui.label(RichText::new("Extensões (csv)").color(theme::TEXT_DIM));
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
                if theme::outline_button(ui, "Adicionar sistema ao rascunho", theme::ACCENT).clicked()
                {
                    self.add_system_to_draft();
                }
            }
            SettingsTab::Controls => {
                theme::section_heading(ui, "Controles", theme::ACCENT);
                ui.label(
                    RichText::new(
                        "Mapeamento de comando e atalhos globais — planejado para versão futura.",
                    )
                    .color(theme::TEXT_DIM),
                );
            }
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
                    "{} ROM(s) no catálogo ({} atualizada(s) no scan, {} com metadata)",
                    self.state.games.len(),
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
        let game_system_key = game.system_key.clone();
        let tx = self.game_done_tx.clone();
        let ctx = ctx.clone();

        std::thread::spawn(move || {
            let result = run_retroarch_blocking(
                &config,
                &rom_path,
                Some(game_system_key.as_str()),
            );
            let _ = tx.send(RetroArchSessionResult { rom_path, result });
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
        self.state.games = self.catalog.list_games()?;
        self.state.recent = self.catalog.list_recent(10)?;
        self.state.recently_added = self.catalog.list_recently_added(16)?;
        self.state.most_played = self.catalog.list_most_played(16)?;
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

}

fn parse_csv_values(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
