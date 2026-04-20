//! Integração com emuladores (RetroArch).

pub mod launcher;

pub use launcher::{RetroArchSessionResult, run_retroarch_blocking};
