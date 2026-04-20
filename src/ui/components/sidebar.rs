use egui::{Button, Color32, RichText, Rounding, Stroke, Ui};

use crate::state::app_state::AppView;
use crate::state::filter_state::{FilterState, SidebarSection};
use crate::theme;

pub struct SidebarCounts {
    pub total: usize,
    pub favorites: usize,
    pub recent: usize,
    pub systems: Vec<(String, usize)>,
}

fn nav_button(ui: &mut Ui, icon: &str, label: &str, count: Option<usize>, active: bool) -> bool {
    let fill = if active {
        theme::ACCENT.linear_multiply(0.2)
    } else {
        theme::UI_CARD_ELEVATED
    };
    let stroke = if active {
        Stroke::new(1.5, theme::ACCENT)
    } else {
        Stroke::new(1.0, Color32::from_white_alpha(18))
    };
    let text = if active {
        theme::TEXT_MAIN
    } else {
        theme::TEXT_DIM
    };

    let line = if let Some(c) = count {
        format!("{}  {:<12} {}", icon, label, c)
    } else {
        format!("{}  {}", icon, label)
    };
    let mut rt = RichText::new(line).color(text);
    if active {
        rt = rt.strong();
    }

    let btn = Button::new(rt)
        .fill(fill)
        .stroke(stroke)
        .rounding(Rounding::same(8.0))
        .min_size(egui::vec2(ui.available_width(), 36.0));

    let clicked = ui.add(btn).clicked();
    ui.add_space(4.0);
    clicked
}

/// Barra lateral: navegação principal + consolas + acesso a configurações.
pub fn render_sidebar(
    ui: &mut Ui,
    filter: &mut FilterState,
    counts: &SidebarCounts,
    selected_view: &AppView,
) -> SidebarNavAction {
    let mut action = SidebarNavAction::None;
    ui.vertical(|ui| {
        // Ocupa toda a altura da coluna (painel até à base da área útil).
        ui.set_min_height(ui.available_height());

        ui.label(
            RichText::new("Navegação")
                .strong()
                .size(13.0)
                .color(theme::TEXT_DIM),
        );
        ui.add_space(10.0);

        let home_active = matches!(selected_view, AppView::Dashboard);
        if nav_button(ui, "⌂", "Home", None, home_active) {
            action = SidebarNavAction::GoHome;
        }

        let all_active =
            matches!(selected_view, AppView::Library) && filter.section == SidebarSection::All;
        if nav_button(
            ui,
            "⊞",
            "Todos",
            Some(counts.total),
            all_active,
        ) {
            action = SidebarNavAction::GoLibraryAll;
        }
        if nav_button(
            ui,
            "★",
            "Favoritos",
            Some(counts.favorites),
            matches!(selected_view, AppView::Library)
                && filter.section == SidebarSection::Favorites,
        ) {
            action = SidebarNavAction::GoFavorites;
        }
        if nav_button(
            ui,
            "⏱",
            "Recentes",
            Some(counts.recent),
            matches!(selected_view, AppView::Library)
                && filter.section == SidebarSection::Recent,
        ) {
            action = SidebarNavAction::GoRecent;
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);
        ui.label(
            RichText::new("Consolas")
                .strong()
                .size(13.0)
                .color(theme::TEXT_DIM),
        );
        ui.add_space(8.0);

        // Lista de consolas: espaço até ao rodapé (3 nav_buttons + folgas).
        const RODAPE_CONFIG: f32 = 152.0;
        let scroll_h = (ui.available_height() - RODAPE_CONFIG).max(48.0);
        egui::ScrollArea::vertical()
            .id_source("sidebar_systems_scroll")
            .max_height(scroll_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (key, n) in &counts.systems {
                    let active = matches!(selected_view, AppView::Library)
                        && matches!(
                            &filter.section,
                            SidebarSection::Console(k) if k.eq_ignore_ascii_case(key)
                        );
                    if nav_button(
                        ui,
                        "◇",
                        &key.to_ascii_uppercase(),
                        Some(*n),
                        active,
                    ) {
                        action = SidebarNavAction::GoConsole(key.clone());
                    }
                }
            });

        // Com o painel alto, empurra o rodapé até à base visual.
        const ALTURA_RODAPE_BTNS: f32 = 132.0;
        let folga = (ui.available_height() - ALTURA_RODAPE_BTNS).max(0.0);
        if folga > 0.0 {
            ui.add_space(folga);
        } else {
            ui.add_space(8.0);
        }

        if nav_button(ui, "↻", "Sincronizar biblioteca", None, false) {
            action = SidebarNavAction::SyncLibrary;
        }
        if nav_button(ui, "▤", "Atualizar metadados", None, false) {
            action = SidebarNavAction::RefreshMetadata;
        }

        let settings_active = matches!(selected_view, AppView::Settings);
        if nav_button(ui, "⚙", "Configurações", None, settings_active) {
            action = SidebarNavAction::GoSettings;
        }
    });
    action
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarNavAction {
    None,
    GoHome,
    GoLibraryAll,
    GoFavorites,
    GoRecent,
    GoConsole(String),
    SyncLibrary,
    RefreshMetadata,
    GoSettings,
}
