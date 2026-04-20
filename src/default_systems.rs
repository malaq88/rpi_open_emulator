//! Defaults de `systems` alinhados aos cores Libretro mais comuns (`.so` em `/usr/lib/libretro` ou
//! pastas equivalentes). O `launcher` procura o ficheiro real por nome aproximado.
//!
//! Chaves = nome da subpasta em `ROMs/<chave>/` e `BIOS/<chave>/`. Um core por plataforma.

use std::collections::HashMap;

use crate::config::SystemConfig;

fn s(core: &str, exts: &[&str]) -> SystemConfig {
    SystemConfig {
        default_core: core.to_string(),
        accepted_extensions: exts.iter().map(|e| (*e).to_string()).collect(),
        extra_args: vec![],
    }
}

/// Mapa de sistemas Libretro para configuração inicial ou merge de chaves em falta.
pub(crate) fn all_default_systems() -> HashMap<String, SystemConfig> {
    [
        ("3do", s("4do_libretro.so", &["iso", "bin", "chd"])),
        ("amstrad", s("caprice32_libretro.so", &["dsk", "sna", "cpr", "kcr"])),
        ("apple2", s("linapple_libretro.so", &["dsk", "do", "po", "nib", "hdv"])),
        ("arcade", s("mame2003_plus_libretro.so", &["zip"])),
        ("atari2600", s("stella_libretro.so", &["a26", "bin"])),
        ("atari5200", s("atari800_libretro.so", &["a52", "bin", "car", "rom", "xfd"])),
        ("atari7800", s("prosystem_libretro.so", &["a78", "bin"])),
        ("atari800", s("atari800_libretro.so", &["atr", "xfd", "bin", "cas", "com"])),
        ("atari_st", s("hatari_libretro.so", &["st", "msa", "stx", "dim", "ipf"])),
        ("c64", s("vice_x64_libretro.so", &["d64", "t64", "tap", "prg", "p00", "crt"])),
        ("c128", s("vice_x128_libretro.so", &["d64", "t64", "tap", "prg", "crt"])),
        ("coleco", s("bluemsx_libretro.so", &["col", "cv", "rom"])),
        ("dos", s("dosbox_pure_libretro.so", &["com", "exe", "bat", "conf"])),
        ("dreamcast", s("flycast_libretro.so", &["gdi", "cdi", "chd", "cue", "bin"])),
        ("gamecube", s("dolphin_libretro.so", &["iso", "gcm", "gcz", "dol", "tgc"])),
        ("gamegear", s("genesis_plus_gx_libretro.so", &["gg", "bin"])),
        ("gb", s("gambatte_libretro.so", &["gb", "sgb"])),
        ("gba", s("mgba_libretro.so", &["gba", "agb"])),
        ("gbc", s("gambatte_libretro.so", &["gbc"])),
        ("genesis", s("genesis_plus_gx_libretro.so", &["md", "smd", "gen", "cue", "iso"])),
        ("intellivision", s("freeintv_libretro.so", &["int", "rom", "bin"])),
        ("jaguar", s("virtualjaguar_libretro.so", &["j64", "abs", "cof", "rom", "bin"])),
        ("lynx", s("handy_libretro.so", &["lnx", "lyx"])),
        ("mastersystem", s("genesis_plus_gx_libretro.so", &["sms", "ms"])),
        ("msx", s("bluemsx_libretro.so", &["rom", "mx1", "mx2", "dsk", "cas", "sc"])),
        ("n64", s("mupen64plus_next_libretro.so", &["n64", "z64", "v64", "ndd", "u64"])),
        ("nds", s("melonds_libretro.so", &["nds"])),
        ("neogeo", s("fbneo_libretro.so", &["neo", "zip", "7z"])),
        ("nes", s("nestopia_libretro.so", &["nes", "fds", "unf", "unif"])),
        ("ngp", s("mednafen_ngp_libretro.so", &["ngp", "ngc"])),
        ("odyssey2", s("o2em_libretro.so", &["o2"])),
        ("pce", s("beetle_pce_fast_libretro.so", &["pce", "cue", "ccd", "iso", "chd"])),
        ("pcfx", s("beetle_pcfx_libretro.so", &["cue", "ccd", "toc", "chd", "iso"])),
        ("pc98", s("nekop2_libretro.so", &["d88", "hdm", "fdi", "fdd", "n88", "98d"])),
        ("pet", s("vice_xpet_libretro.so", &["prg", "tap", "d64", "t64"])),
        ("psx", s("pcsx_rearmed_libretro.so", &["cue", "bin", "img", "mdf", "pbp", "chd", "ecm", "m3u"])),
        ("ps2", s("play_libretro.so", &["iso", "chd", "bin", "mdf"])),
        ("pokemon_mini", s("pokemini_libretro.so", &["minc"])),
        ("psp", s("ppsspp_libretro.so", &["iso", "cso", "pbp", "elf"])),
        ("quake", s("tyrquake_libretro.so", &["pak", "pk3"])),
        ("saturn", s("beetle_saturn_libretro.so", &["cue", "chd", "iso", "mds", "ccd", "m3u"])),
        ("scummvm", s("scummvm_libretro.so", &["svm", "scummvm"])),
        ("sega32x", s("picodrive_libretro.so", &["32x", "smd", "md", "bin"])),
        ("segacd", s("genesis_plus_gx_libretro.so", &["cue", "chd", "iso", "mds", "ccd"])),
        ("sg1000", s("genesis_plus_gx_libretro.so", &["sg", "sc"])),
        ("snes", s("snes9x_libretro.so", &["smc", "sfc", "swc", "fig", "bs"])),
        ("supergrafx", s("beetle_sgx_libretro.so", &["sgx", "pce", "cue", "chd"])),
        ("vectrex", s("vecx_libretro.so", &["vec", "gam", "bin"])),
        ("vic20", s("vice_xvic_libretro.so", &["prg", "tap", "crt", "d64", "t64"])),
        ("virtualboy", s("beetle_vb_libretro.so", &["vb", "vboy"])),
        ("wii", s("dolphin_libretro.so", &["iso", "wbfs", "ciso", "gcz", "wad", "dol"])),
        ("wonderswan", s("mednafen_wswan_libretro.so", &["ws", "wsc"])),
        ("x68000", s("px68k_libretro.so", &["dim", "zip", "xdf", "2hd", "d88"])),
        ("zx81", s("81_libretro.so", &["p", "81", "o", "zip"])),
        ("zxspectrum", s("fuse_libretro.so", &["tap", "tzx", "z80", "scl", "trd", "dsk"])),
        ("thomson", s("theodore_libretro.so", &["fd", "sap", "k7", "m7", "m5", "dsk"])),
        ("easyrpg", s("easyrpg_libretro.so", &["ldb", "ini", "zip"])),
        ("gw", s("gw_libretro.so", &["mgw"])),
        ("prboom", s("prboom_libretro.so", &["wad", "iwad", "pwad"])),
        ("3ds", s("citra_libretro.so", &["3ds", "cci", "cxi", "app", "axf"])),
        ("vmu", s("vemulator_libretro.so", &["vms", "dcm"])),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect()
}
