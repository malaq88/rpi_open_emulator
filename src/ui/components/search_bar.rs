use egui::{RichText, Ui};

use crate::theme;

/// Campo de busca com placeholder.
pub fn render_search_bar(ui: &mut Ui, query: &mut String, id: egui::Id) -> egui::Response {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("🔎")
                .size(16.0)
                .color(theme::TEXT_DIM),
        );
        ui.add_space(8.0);
        ui.add_sized(
            [ui.available_width().max(60.0), 36.0],
            egui::TextEdit::singleline(query)
                .id(id)
                .hint_text(RichText::new("Buscar jogos…").color(theme::TEXT_DIM))
                .font(egui::TextStyle::Body),
        )
    })
    .inner
}
