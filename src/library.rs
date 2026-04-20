use std::{fs, path::PathBuf};

use crate::config::LauncherConfig;

#[derive(Debug, Clone)]
pub struct RomEntry {
    pub path: PathBuf,
    pub file_name: String,
    pub extension: String,
    pub system_key: Option<String>,
}

pub fn scan_roms(config: &LauncherConfig) -> anyhow::Result<Vec<RomEntry>> {
    if !config.library.roms_dir.exists() {
        return Ok(vec![]);
    }

    let mut entries = vec![];
    let read_dir = fs::read_dir(&config.library.roms_dir)?;

    for item in read_dir {
        let path = item?.path();
        if !path.is_file() {
            continue;
        }

        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase());

        let Some(extension) = extension else {
            continue;
        };

        let system_key = config.resolve_system_key_for_extension(&extension);
        if system_key.is_none() {
            continue;
        }

        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        entries.push(RomEntry {
            path,
            file_name,
            extension,
            system_key,
        });
    }

    entries.sort_by(|left, right| left.file_name.cmp(&right.file_name));
    Ok(entries)
}
