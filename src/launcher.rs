use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, bail};

use crate::config::{LauncherConfig, SystemConfig};

fn resolve_core_path(
    config: &LauncherConfig,
    system_key: &str,
    system: &SystemConfig,
) -> anyhow::Result<PathBuf> {
    let configured_core = Path::new(&system.default_core);
    if configured_core.is_absolute() {
        if configured_core.exists() {
            return Ok(configured_core.to_path_buf());
        }
        bail!("core configurado nao existe: {}", configured_core.display());
    }

    let default_candidate = config.retroarch.cores_dir.join(configured_core);
    if default_candidate.exists() {
        return Ok(default_candidate);
    }

    let search_dirs = build_core_search_dirs(&config.retroarch.cores_dir);

    for dir in &search_dirs {
        let candidate = dir.join(configured_core);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    let configured_stem = system
        .default_core
        .strip_suffix("_libretro.so")
        .unwrap_or(&system.default_core)
        .to_ascii_lowercase();

    for dir in &search_dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let file_path = entry.path();
            let file_name = file_path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_ascii_lowercase());

            if let Some(file_name) = file_name {
                let is_libretro = file_name.ends_with("_libretro.so");
                let close_match = file_name.contains(&configured_stem);
                if is_libretro && close_match {
                    return Ok(file_path);
                }
            }
        }
    }

    let inspected = search_dirs
        .iter()
        .map(|d| d.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let install_hint = package_hint_for_system(system_key);

    bail!(
        "nao foi possivel localizar o core {} (buscado em: {}). {}",
        system.default_core,
        inspected,
        install_hint
    )
}

fn build_core_search_dirs(primary: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![];
    push_if_exists_unique(&mut dirs, primary.to_path_buf());

    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        push_if_exists_unique(&mut dirs, home.join(".config/retroarch/cores"));
        push_if_exists_unique(&mut dirs, home.join(".local/share/retroarch/cores"));
        push_if_exists_unique(
            &mut dirs,
            home.join(".var/app/org.libretro.RetroArch/config/retroarch/cores"),
        );
    }

    let fixed = [
        PathBuf::from("/usr/lib/libretro"),
        PathBuf::from("/usr/local/lib/libretro"),
        PathBuf::from("/usr/lib/x86_64-linux-gnu/libretro"),
        PathBuf::from("/usr/lib/aarch64-linux-gnu/libretro"),
        PathBuf::from("/usr/lib/arm-linux-gnueabihf/libretro"),
        PathBuf::from("/var/lib/snapd/hostfs/usr/lib/libretro"),
    ];

    for candidate in fixed {
        push_if_exists_unique(&mut dirs, candidate);
    }

    dirs
}

fn push_if_exists_unique(dirs: &mut Vec<PathBuf>, candidate: PathBuf) {
    if candidate.exists() && !dirs.contains(&candidate) {
        dirs.push(candidate);
    }
}

fn resolve_retroarch_binary(config: &LauncherConfig) -> anyhow::Result<PathBuf> {
    let configured = config.retroarch.binary_path.clone();
    if configured.is_absolute() {
        if configured.exists() {
            return Ok(configured);
        }
        bail!(
            "binario RetroArch nao encontrado em {}",
            configured.display()
        );
    }

    if let Some(found) = find_executable_in_path(
        configured
            .to_str()
            .filter(|s| !s.is_empty())
            .unwrap_or("retroarch"),
    ) {
        return Ok(found);
    }

    bail!(
        "comando '{}' nao encontrado no PATH",
        configured.display()
    )
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

fn package_hint_for_system(system_key: &str) -> &'static str {
    match system_key {
        "snes" => {
            "Instale um core SNES (ex.: pacote 'libretro-snes9x') ou ajuste systems.snes.default_core para um core instalado."
        }
        "nes" => {
            "Instale um core NES (ex.: 'libretro-nestopia') ou ajuste systems.nes.default_core para um core instalado."
        }
        "gba" => {
            "Instale um core GBA (ex.: 'libretro-mgba') ou ajuste systems.gba.default_core para um core instalado."
        }
        _ => "Instale o core correspondente ou ajuste default_core para um arquivo existente.",
    }
}

pub fn launch_game(config: &LauncherConfig, rom_path: &Path) -> anyhow::Result<()> {
    let extension = rom_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .context("arquivo ROM sem extensao reconhecivel")?;

    let system_key = config
        .resolve_system_key_for_extension(&extension)
        .with_context(|| format!("nenhum sistema configurado para extensao .{}", extension))?;

    let system = config
        .systems
        .get(&system_key)
        .with_context(|| format!("sistema {} nao encontrado na configuracao", system_key))?;

    let core_path = resolve_core_path(config, &system_key, system)?;
    let retroarch_binary = resolve_retroarch_binary(config)?;
    let mut command = Command::new(&retroarch_binary);

    command.arg("-L").arg(&core_path);
    command.args(&config.retroarch.extra_args);
    command.args(&system.extra_args);
    command.arg(rom_path);

    let status = command.status().context("falha ao iniciar RetroArch")?;
    if !status.success() {
        bail!("RetroArch retornou codigo {:?}", status.code());
    }

    Ok(())
}
