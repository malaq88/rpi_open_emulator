use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use rusqlite::{Connection, params};

use crate::config::LauncherConfig;

const GAME_ROW_SELECT: &str = "
                g.path,
                g.file_name,
                g.extension,
                g.system_key,
                COALESCE(m.title, g.file_name) AS title,
                m.description,
                m.cover_path,
                COALESCE(m.source, 'scanner') AS source,
                g.is_favorite,
                g.play_count,
                g.last_played_at,
                m.genre,
                m.release_year,
                m.developer
             FROM games g
             LEFT JOIN game_metadata m ON m.game_path = g.path";

#[derive(Debug, Clone)]
pub struct GameEntry {
    pub path: PathBuf,
    pub file_name: String,
    pub extension: String,
    pub system_key: String,
    pub title: String,
    pub description: Option<String>,
    pub cover_path: Option<PathBuf>,
    pub metadata_source: String,
    pub is_favorite: bool,
    pub play_count: i64,
    pub last_played_at: Option<i64>,
    pub genre: Option<String>,
    pub release_year: Option<i32>,
    pub developer: Option<String>,
}

pub struct Catalog {
    conn: Connection,
}

impl Catalog {
    pub fn open(db_path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("falha ao criar diretorio do catalogo {}", parent.display())
            })?;
        }

        let conn = Connection::open(db_path)
            .with_context(|| format!("falha ao abrir banco {}", db_path.display()))?;
        let catalog = Self { conn };
        catalog.init_schema()?;
        Ok(catalog)
    }

    pub fn sync_with_filesystem(&mut self, config: &LauncherConfig) -> anyhow::Result<usize> {
        if !config.library.roms_dir.exists() {
            return Ok(0);
        }

        let scan_id = now_unix_secs();
        let mut scanned_count: usize = 0;
        let root = config.library.roms_dir.clone();

        for (system_key, root_dir) in config.rom_scan_pairs_sorted() {
            let Some(system_cfg) = config.systems.get(&system_key) else {
                continue;
            };
            if !root_dir.is_dir() {
                continue;
            }

            let mut stack = vec![root_dir];

            while let Some(current_dir) = stack.pop() {
                let read_dir = fs::read_dir(&current_dir)
                    .with_context(|| format!("falha ao ler diretorio {}", current_dir.display()))?;

                for entry in read_dir {
                    let path = entry?.path();
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
                        .any(|candidate| candidate.eq_ignore_ascii_case(&extension));
                    if !accepted {
                        continue;
                    }

                    let metadata = fs::metadata(&path).with_context(|| {
                        format!("falha ao obter metadados da ROM {}", path.display())
                    })?;
                    let modified_unix = metadata
                        .modified()
                        .ok()
                        .and_then(to_unix_secs)
                        .unwrap_or(scan_id);
                    let size_bytes = i64::try_from(metadata.len()).unwrap_or(i64::MAX);
                    let path_text = path.to_string_lossy().to_string();
                    let file_name = file_name_for_path(&path);

                    self.conn.execute(
                        "INSERT INTO games (
                        path,
                        file_name,
                        extension,
                        system_key,
                        modified_unix,
                        size_bytes,
                        last_scan_id
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                    ON CONFLICT(path) DO UPDATE SET
                        file_name = excluded.file_name,
                        extension = excluded.extension,
                        system_key = excluded.system_key,
                        modified_unix = excluded.modified_unix,
                        size_bytes = excluded.size_bytes,
                        last_scan_id = excluded.last_scan_id",
                        params![
                            path_text,
                            file_name,
                            extension,
                            system_key,
                            modified_unix,
                            size_bytes,
                            scan_id
                        ],
                    )?;
                    self.upsert_offline_metadata(
                        &path,
                        &file_name,
                        &system_key,
                        &config.library.roms_dir,
                    )?;
                    scanned_count += 1;
                }
            }
        }

        let root_prefix = format!("{}/%", root.to_string_lossy().trim_end_matches('/'));
        self.conn.execute(
            "DELETE FROM games WHERE path LIKE ?1 AND last_scan_id <> ?2",
            params![root_prefix, scan_id],
        )?;
        self.conn.execute(
            "DELETE FROM game_metadata WHERE game_path NOT IN (SELECT path FROM games)",
            [],
        )?;

        Ok(scanned_count)
    }

    pub fn refresh_metadata_cache(&mut self, config: &LauncherConfig) -> anyhow::Result<usize> {
        let games = self.list_games()?;
        for game in &games {
            self.upsert_offline_metadata(
                &game.path,
                &game.file_name,
                &game.system_key,
                &config.library.roms_dir,
            )?;
        }
        Ok(games.len())
    }

    pub fn list_games(&self) -> anyhow::Result<Vec<GameEntry>> {
        let sql = format!(
            "SELECT {} ORDER BY file_name COLLATE NOCASE ASC",
            GAME_ROW_SELECT
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], row_to_game)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_favorites(&self) -> anyhow::Result<Vec<GameEntry>> {
        let sql = format!(
            "SELECT {} WHERE g.is_favorite = 1 ORDER BY file_name COLLATE NOCASE ASC",
            GAME_ROW_SELECT
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], row_to_game)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_recent(&self, limit: i64) -> anyhow::Result<Vec<GameEntry>> {
        let sql = format!(
            "SELECT {}
             WHERE g.last_played_at IS NOT NULL
             ORDER BY g.last_played_at DESC
             LIMIT ?1",
            GAME_ROW_SELECT
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![limit], row_to_game)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_recently_added(&self, limit: i64) -> anyhow::Result<Vec<GameEntry>> {
        let sql = format!(
            "SELECT {}
             ORDER BY g.modified_unix DESC
             LIMIT ?1",
            GAME_ROW_SELECT
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![limit], row_to_game)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_most_played(&self, limit: i64) -> anyhow::Result<Vec<GameEntry>> {
        let sql = format!(
            "SELECT {}
             WHERE g.play_count > 0
             ORDER BY g.play_count DESC,
                      (g.last_played_at IS NULL) ASC,
                      g.last_played_at DESC
             LIMIT ?1",
            GAME_ROW_SELECT
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![limit], row_to_game)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn set_favorite(&self, rom_path: &Path, value: bool) -> anyhow::Result<()> {
        self.conn.execute(
            "UPDATE games SET is_favorite = ?2 WHERE path = ?1",
            params![rom_path.to_string_lossy().to_string(), value as i64],
        )?;
        Ok(())
    }

    pub fn mark_played(&self, rom_path: &Path) -> anyhow::Result<()> {
        self.conn.execute(
            "UPDATE games
             SET play_count = play_count + 1,
                 last_played_at = ?2
             WHERE path = ?1",
            params![rom_path.to_string_lossy().to_string(), now_unix_secs()],
        )?;
        Ok(())
    }

    fn init_schema(&self) -> anyhow::Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS games (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                file_name TEXT NOT NULL,
                extension TEXT NOT NULL,
                system_key TEXT NOT NULL,
                is_favorite INTEGER NOT NULL DEFAULT 0,
                play_count INTEGER NOT NULL DEFAULT 0,
                last_played_at INTEGER,
                modified_unix INTEGER NOT NULL DEFAULT 0,
                size_bytes INTEGER NOT NULL DEFAULT 0,
                last_scan_id INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS game_metadata (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                game_path TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                description TEXT,
                genre TEXT,
                cover_path TEXT,
                source TEXT NOT NULL DEFAULT 'scanner',
                updated_at INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_games_system_key ON games(system_key);
            CREATE INDEX IF NOT EXISTS idx_games_favorite ON games(is_favorite);
            CREATE INDEX IF NOT EXISTS idx_games_recent ON games(last_played_at);
            CREATE INDEX IF NOT EXISTS idx_metadata_game_path ON game_metadata(game_path);",
        )?;
        self.ensure_metadata_columns()?;
        Ok(())
    }

    fn ensure_metadata_columns(&self) -> anyhow::Result<()> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(game_metadata)")?;
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?;
        if !cols.iter().any(|c| c == "release_year") {
            self.conn
                .execute("ALTER TABLE game_metadata ADD COLUMN release_year INTEGER", [])?;
        }
        if !cols.iter().any(|c| c == "developer") {
            self.conn
                .execute("ALTER TABLE game_metadata ADD COLUMN developer TEXT", [])?;
        }
        Ok(())
    }

    fn upsert_offline_metadata(
        &self,
        rom_path: &Path,
        file_name: &str,
        system_key: &str,
        rom_root: &Path,
    ) -> anyhow::Result<()> {
        let title = title_from_file_name(file_name);
        let cover_path = discover_cover_for_rom(rom_path, system_key, rom_root)
            .map(|path| path.to_string_lossy().to_string());
        let path_text = rom_path.to_string_lossy().to_string();

        self.conn.execute(
            "INSERT INTO game_metadata (
                game_path,
                title,
                description,
                genre,
                cover_path,
                source,
                updated_at
            ) VALUES (?1, ?2, NULL, NULL, ?3, 'offline-cache', ?4)
            ON CONFLICT(game_path) DO UPDATE SET
                title = excluded.title,
                cover_path = COALESCE(excluded.cover_path, game_metadata.cover_path),
                source = 'offline-cache',
                updated_at = excluded.updated_at",
            params![path_text, title, cover_path, now_unix_secs()],
        )?;

        Ok(())
    }
}

fn row_to_game(row: &rusqlite::Row<'_>) -> rusqlite::Result<GameEntry> {
    let path_text: String = row.get(0)?;
    let cover_path_text: Option<String> = row.get(6)?;
    let is_favorite_int: i64 = row.get(8)?;
    Ok(GameEntry {
        path: PathBuf::from(path_text),
        file_name: row.get(1)?,
        extension: row.get(2)?,
        system_key: row.get(3)?,
        title: row.get(4)?,
        description: row.get(5)?,
        cover_path: cover_path_text.map(PathBuf::from),
        metadata_source: row.get(7)?,
        is_favorite: is_favorite_int != 0,
        play_count: row.get(9)?,
        last_played_at: row.get(10)?,
        genre: row.get(11)?,
        release_year: row.get(12)?,
        developer: row.get(13)?,
    })
}

fn now_unix_secs() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_secs()).unwrap_or(i64::MAX),
        Err(_) => 0,
    }
}

fn to_unix_secs(system_time: SystemTime) -> Option<i64> {
    system_time
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
}

fn file_name_for_path(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

fn title_from_file_name(file_name: &str) -> String {
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(file_name);

    let mut clean = String::with_capacity(stem.len());
    let mut skip_depth: usize = 0;
    for ch in stem.chars() {
        match ch {
            '(' | '[' => skip_depth += 1,
            ')' | ']' => skip_depth = skip_depth.saturating_sub(1),
            _ if skip_depth == 0 => clean.push(ch),
            _ => {}
        }
    }

    clean
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn discover_cover_for_rom(rom_path: &Path, system_key: &str, rom_root: &Path) -> Option<PathBuf> {
    let stem = rom_path.file_stem().and_then(|value| value.to_str())?;
    let same_dir = rom_path.parent()?;
    let image_extensions = ["png", "jpg", "jpeg", "webp"];

    for ext in image_extensions {
        let candidate = same_dir.join(format!("{stem}.{ext}"));
        if candidate.exists() {
            return Some(candidate);
        }
    }

    let covers_root = rom_root.join("covers");
    for ext in image_extensions {
        let by_system = covers_root.join(system_key).join(format!("{stem}.{ext}"));
        if by_system.exists() {
            return Some(by_system);
        }
    }

    for ext in image_extensions {
        let generic = covers_root.join(format!("{stem}.{ext}"));
        if generic.exists() {
            return Some(generic);
        }
    }

    None
}
