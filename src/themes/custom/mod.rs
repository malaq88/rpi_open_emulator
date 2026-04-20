//! Temas carregados pelo utilizador (futuro: ficheiros TOML/JSON em `~/.config/.../themes/`).
//!
//! Por agora apenas documentação de extensão — ver `plugins::api::ThemePlugin`.

pub fn user_theme_dir_hint() -> &'static str {
    "Coloque temas custom no diretório de dados da app (futuro)."
}
