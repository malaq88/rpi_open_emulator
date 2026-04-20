use std::{fs, path::PathBuf};

use crate::config::LauncherConfig;

#[derive(Debug, Clone)]
pub struct RomEntry {
    pub path: PathBuf,
    pub file_name: String,
    pub extension: String,
    pub system_key: Option<String>,
}

/// Lista ROMs em `<roms_dir>/<sistema>/` (recursivo), respeitando as extensões de cada sistema.
pub fn scan_roms(config: &LauncherConfig) -> Result<Vec<RomEntry>, anyhow::Error> {
    if !config.library.roms_dir.exists() {
        return Ok(vec![]);
    }

    let mut entries = vec![];

    for (system_key, root_dir) in config.rom_scan_pairs_sorted() {
        let Some(system_cfg) = config.systems.get(&system_key) else {
            continue;
        };
        if !root_dir.is_dir() {
            continue;
        }

        let mut stack = vec![root_dir];
        while let Some(dir) = stack.pop() {
            let read_dir = match fs::read_dir(&dir) {
                Ok(r) => r,
                Err(_) => continue,
            };

            for item in read_dir.flatten() {
                let path = item.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
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

                let accepted = system_cfg
                    .accepted_extensions
                    .iter()
                    .any(|e| e.eq_ignore_ascii_case(&extension));
                if !accepted {
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
                    system_key: Some(system_key.clone()),
                });
            }
        }
    }

    entries.sort_by(|left, right| left.file_name.cmp(&right.file_name));
    Ok(entries)
}
