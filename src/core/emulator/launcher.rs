use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
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

/// Resultado de uma sessão RetroArch executada fora da thread da UI.
#[derive(Debug)]
pub struct RetroArchSessionResult {
    pub rom_path: PathBuf,
    pub result: anyhow::Result<ExitStatus>,
}

fn path_for_retroarch_system_cfg(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .replace('"', "\\\"")
}

/// `$HOME` em Unix; em Windows costuma ser `%USERPROFILE%`.
fn user_home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
}

/// Caminhos habituais do `retroarch.cfg` do utilizador (Linux + Flatpak + Windows).
fn retroarch_main_cfg_candidates() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(home) = user_home_dir() {
        v.push(home.join(".config/retroarch/retroarch.cfg"));
        v.push(
            home.join(".var/app/org.libretro.RetroArch/config/retroarch/retroarch.cfg"),
        );
        #[cfg(windows)]
        v.push(home.join("AppData/Roaming/RetroArch/retroarch.cfg"));
    }
    v
}

fn first_existing_retroarch_main_cfg() -> Option<PathBuf> {
    retroarch_main_cfg_candidates()
        .into_iter()
        .find(|p| p.is_file())
}

/// Pastas com perfis `*.cfg` do autoconfig (ordem: preferir config do utilizador, depois sistema).
fn joypad_autoconfig_candidate_dirs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(home) = user_home_dir() {
        v.push(home.join(".config/retroarch/autoconfig"));
        v.push(
            home.join(".var/app/org.libretro.RetroArch/config/retroarch/autoconfig"),
        );
        #[cfg(windows)]
        v.push(home.join("AppData/Roaming/RetroArch/autoconfig"));
    }
    #[cfg(target_os = "linux")]
    {
        v.extend([
            PathBuf::from("/usr/share/libretro/autoconfig"),
            PathBuf::from("/usr/share/retroarch/autoconfig"),
            PathBuf::from("/usr/lib/libretro/autoconfig"),
            PathBuf::from("/usr/local/share/libretro/autoconfig"),
        ]);
    }
    v
}

fn first_existing_autoconfig_dir() -> Option<PathBuf> {
    joypad_autoconfig_candidate_dirs()
        .into_iter()
        .find(|p| p.is_dir())
}

/// Trechos extra para `--appendconfig`: deteção de comandos + diretório de perfis conhecido.
fn build_appendconfig_snippet(bios_escaped: &str) -> String {
    let mut cfg = format!("system_directory = \"{bios_escaped}\"\n");
    // Liga autoconfig por perfis (vendor/product/device); requer ficheiros em joypad_autoconfig_dir.
    cfg.push_str("input_autodetect_enable = \"true\"\n");
    if let Some(dir) = first_existing_autoconfig_dir() {
        let esc = path_for_retroarch_system_cfg(&dir);
        cfg.push_str(&format!("joypad_autoconfig_dir = \"{esc}\"\n"));
    }
    cfg
}

fn build_retroarch_command(
    config: &LauncherConfig,
    rom_path: &Path,
    system_key_override: Option<&str>,
) -> anyhow::Result<(Command, PathBuf)> {
    let extension = rom_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .context("arquivo ROM sem extensao reconhecivel")?;

    let system_key = if let Some(key) = system_key_override {
        let key_norm = key.trim().to_ascii_lowercase();
        if !config.systems.contains_key(&key_norm) {
            bail!("sistema '{}' nao existe na configuracao", key_norm);
        }
        key_norm
    } else {
        config
            .resolve_system_key_for_extension(&extension)
            .with_context(|| format!("nenhum sistema configurado para extensao .{}", extension))?
    };

    let expected_rom_root = config.rom_dir_for_system(&system_key);
    if !rom_path.starts_with(&expected_rom_root) {
        bail!(
            "ROM deve estar em {} (subpasta do sistema '{}')",
            expected_rom_root.display(),
            system_key
        );
    }

    let system = config
        .systems
        .get(&system_key)
        .with_context(|| format!("sistema {} nao encontrado na configuracao", system_key))?;

    let core_path = resolve_core_path(config, &system_key, system)?;
    let retroarch_binary = resolve_retroarch_binary(config)?;

    let bios_dir = config.bios_dir_for_system(&system_key);
    fs::create_dir_all(&bios_dir).with_context(|| {
        format!(
            "nao foi possivel criar a pasta de BIOS do sistema: {}",
            bios_dir.display()
        )
    })?;

    let cfg_path = env::temp_dir().join(format!(
        "rpi_open_emulator_retroarch_{}.cfg",
        std::process::id()
    ));
    let bios_escaped = path_for_retroarch_system_cfg(&bios_dir);
    let append_snippet = build_appendconfig_snippet(&bios_escaped);
    fs::write(&cfg_path, &append_snippet)
        .with_context(|| format!("gravar appendconfig em {}", cfg_path.display()))?;

    let mut command = Command::new(&retroarch_binary);
    if let Some(main_cfg) = first_existing_retroarch_main_cfg() {
        command.arg("--config").arg(main_cfg);
    }
    command.arg("--appendconfig").arg(&cfg_path);
    command.arg("-L").arg(&core_path);
    // Inicia em tela cheia; argumentos extra em `retroarch.extra_args` podem sobrescrever (ex.: `--windowed`).
    command.arg("--fullscreen");
    command.args(&config.retroarch.extra_args);
    command.args(&system.extra_args);
    command.arg(rom_path);

    Ok((command, cfg_path))
}

#[cfg(target_os = "linux")]
fn configure_parent_death_signal(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    unsafe {
        command.pre_exec(|| {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(not(target_os = "linux"))]
fn configure_parent_death_signal(_command: &mut Command) {}

pub fn run_retroarch_blocking(
    config: &LauncherConfig,
    rom_path: &Path,
    system_key_override: Option<&str>,
) -> anyhow::Result<ExitStatus> {
    let (mut command, cfg_path) = build_retroarch_command(config, rom_path, system_key_override)?;
    configure_parent_death_signal(&mut command);
    let outcome = (|| -> anyhow::Result<ExitStatus> {
        let mut child = command.spawn().context("falha ao iniciar RetroArch")?;
        child.wait().context("falha ao aguardar RetroArch")
    })();
    let _ = fs::remove_file(&cfg_path);
    outcome
}
