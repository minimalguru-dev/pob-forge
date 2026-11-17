//! Follows the game's own log (Client.txt) for the overlay: the zone, its area level and act, and
//! the character's level. Only the log file is read; it is the file the game writes for players.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::game::Game;

const POLL: Duration = Duration::from_millis(500);
const RESOLVE_EVERY: Duration = Duration::from_secs(10);
/// How much of the end of the log to read on start, to know where the player already is.
const BACKFILL: u64 = 512 * 1024;
pub const UPDATE: &str = "gamelog:update";

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LogState {
    pub path: Option<String>,
    pub zone: Option<String>,
    pub area_id: Option<String>,
    pub area_level: Option<u32>,
    /// Campaign act of the last generated area that has one; endgame areas set `endgame`.
    pub act: Option<u32>,
    pub endgame: bool,
    pub character: Option<String>,
    pub class_name: Option<String>,
    pub level: Option<u32>,
}

#[derive(Default)]
pub struct GameLog {
    state: Mutex<LogState>,
}

/// The campaign act an area id belongs to: PoE1 `1_5_town` / `2_6_1`, PoE2 `G1_town` / `C_G2_1`.
pub fn act_of(area_id: &str) -> Option<u32> {
    let id = area_id.trim_start_matches("C_");
    let mut parts = id.split('_');
    let first = parts.next()?;
    if first == "1" || first == "2" {
        return parts.next()?.parse().ok().filter(|a| (1..=10).contains(a));
    }
    first.strip_prefix('G')?.parse().ok().filter(|a| (1..=10).contains(a))
}

fn is_endgame(area_id: &str) -> bool {
    area_id.starts_with("Map") || area_id.starts_with("MapWorlds")
}

/// Applies one log line. Returns whether anything the overlay shows changed.
pub fn apply(state: &mut LogState, line: &str) -> bool {
    let before = state.clone();
    if let Some(rest) = line.split("Generating level ").nth(1) {
        let mut it = rest.splitn(2, " area \"");
        if let (Some(level), Some(tail)) = (it.next(), it.next()) {
            if let (Ok(level), Some(id)) = (level.trim().parse::<u32>(), tail.split('"').next()) {
                state.area_id = Some(id.to_string());
                state.area_level = Some(level);
                if let Some(act) = act_of(id) {
                    state.act = Some(act);
                    state.endgame = false;
                } else if is_endgame(id) {
                    state.endgame = true;
                }
                // Hideouts, the labyrinth and league areas keep the act they were entered from.
            }
        }
    } else if let Some(msg) = line.split("] : ").nth(1) {
        let msg = msg.trim_end();
        if let Some(zone) = msg.strip_prefix("You have entered ").and_then(|z| z.strip_suffix('.')) {
            state.zone = Some(zone.to_string());
        } else if let Some((who, level)) = msg.rsplit_once(" is now level ") {
            if let (Some((name, class)), Ok(level)) = (who.rsplit_once(" ("), level.trim().parse::<u32>()) {
                state.character = Some(name.to_string());
                state.class_name = Some(class.trim_end_matches(')').to_string());
                state.level = Some(level);
            }
        }
    }
    *state != before
}

fn steam_libraries() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for root in [r"C:\Program Files (x86)\Steam", r"C:\Program Files\Steam"] {
        let root = PathBuf::from(root);
        if let Ok(text) = std::fs::read_to_string(root.join(r"steamapps\libraryfolders.vdf")) {
            for line in text.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("\"path\"") {
                    let path = rest.trim().trim_matches('"').replace("\\\\", "\\");
                    out.push(PathBuf::from(path));
                }
            }
        }
        out.push(root);
    }
    for drive in ['C', 'D', 'E', 'F', 'G'] {
        out.push(PathBuf::from(format!(r"{drive}:\SteamLibrary")));
    }
    out
}

/// Where the game writes its log, newest first.
pub fn candidates(game: Game) -> Vec<PathBuf> {
    let folder = match game {
        Game::Poe1 => "Path of Exile",
        Game::Poe2 => "Path of Exile 2",
    };
    let mut paths: Vec<PathBuf> = vec![
        PathBuf::from(format!(r"C:\Program Files (x86)\Grinding Gear Games\{folder}\logs\Client.txt")),
        PathBuf::from(format!(r"C:\Program Files\Grinding Gear Games\{folder}\logs\Client.txt")),
    ];
    for lib in steam_libraries() {
        paths.push(lib.join(format!(r"steamapps\common\{folder}\logs\Client.txt")));
    }
    if let Some(home) = std::env::var_os("HOME") {
        // Linux and macOS through Steam's Proton or the native client.
        let home = PathBuf::from(home);
        paths.push(home.join(format!(".steam/steam/steamapps/common/{folder}/logs/Client.txt")));
        paths.push(home.join(format!(".local/share/Steam/steamapps/common/{folder}/logs/Client.txt")));
    }
    let mut found: Vec<(PathBuf, std::time::SystemTime)> = paths
        .into_iter()
        .filter_map(|p| std::fs::metadata(&p).ok().filter(|m| m.is_file()).map(|m| (p, m.modified().unwrap_or(std::time::UNIX_EPOCH))))
        .collect();
    found.sort_by(|a, b| b.1.cmp(&a.1));
    found.dedup_by(|a, b| a.0 == b.0);
    found.into_iter().map(|(p, _)| p).collect()
}

fn resolve(app: &AppHandle) -> Option<PathBuf> {
    let settings = crate::game::load_settings(app).overlay;
    if let Some(p) = settings.log_path.filter(|p| !p.trim().is_empty()) {
        return Some(PathBuf::from(p));
    }
    let game = app.try_state::<crate::AppState>()?.game();
    candidates(game).into_iter().next()
}

/// Reads from `offset` to the end, applying whole lines; returns the new offset.
fn read_from(path: &Path, offset: u64, state: &mut LogState) -> std::io::Result<(u64, bool)> {
    let mut file = File::open(path)?;
    let len = file.metadata()?.len();
    let start = if len < offset { 0 } else { offset };
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::new();
    file.take(len - start).read_to_end(&mut bytes)?;
    // Keep a partial last line for the next read.
    let end = bytes.iter().rposition(|&b| b == b'\n').map(|i| i + 1).unwrap_or(0);
    let mut changed = false;
    for line in String::from_utf8_lossy(&bytes[..end]).lines() {
        changed |= apply(state, line);
    }
    Ok((start + end as u64, changed))
}

pub fn setup(app: &AppHandle) {
    app.manage(GameLog::default());
    let app = app.clone();
    std::thread::Builder::new()
        .name("gamelog".into())
        .spawn(move || {
            let mut current: Option<PathBuf> = None;
            let mut offset = 0u64;
            let mut resolved_at: Option<Instant> = None;
            loop {
                std::thread::sleep(POLL);
                let enabled = crate::game::load_settings(&app).overlay;
                if !enabled.enabled {
                    current = None;
                    resolved_at = None;
                    continue;
                }
                if resolved_at.is_none_or(|t| t.elapsed() >= RESOLVE_EVERY) {
                    resolved_at = Some(Instant::now());
                    let next = resolve(&app);
                    if next != current {
                        current = next;
                        let mut fresh = LogState { path: current.as_ref().map(|p| p.display().to_string()), ..LogState::default() };
                        // Start near the end: a long-played log is hundreds of megabytes.
                        offset = current.as_ref().and_then(|p| std::fs::metadata(p).ok()).map(|m| m.len().saturating_sub(BACKFILL)).unwrap_or(0);
                        if let Some(p) = &current {
                            if let Ok((o, _)) = read_from(p, offset, &mut fresh) {
                                offset = o;
                            }
                        }
                        let gl = app.state::<GameLog>();
                        *gl.state.lock().unwrap_or_else(|e| e.into_inner()) = fresh.clone();
                        let _ = app.emit(UPDATE, &fresh);
                        continue;
                    }
                }
                let Some(path) = &current else { continue };
                let gl = app.state::<GameLog>();
                let mut state = gl.state.lock().unwrap_or_else(|e| e.into_inner()).clone();
                match read_from(path, offset, &mut state) {
                    Ok((o, changed)) => {
                        offset = o;
                        if changed {
                            *gl.state.lock().unwrap_or_else(|e| e.into_inner()) = state.clone();
                            let _ = app.emit(UPDATE, &state);
                        }
                    }
                    Err(e) => log::debug!("gamelog: {}: {e}", path.display()),
                }
            }
        })
        .expect("spawn gamelog reader");
}

#[tauri::command]
pub fn gamelog_state(app: AppHandle) -> LogState {
    app.state::<GameLog>().state.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// The log files found for the active game, newest first, for the Settings page.
#[tauri::command]
pub fn gamelog_candidates(app: AppHandle) -> Vec<String> {
    let game = app.state::<crate::AppState>().game();
    candidates(game).into_iter().map(|p| p.display().to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Lines as Client.txt writes them (PoE1 3.x).
    const AREA: &str = r#"2025/11/26 19:06:04 617306359 1186a8e1 [DEBUG Client 22284] Generating level 45 area "1_5_town" with seed 1"#;
    const ENTER: &str = "2025/11/26 19:06:06 617308500 cff945bb [INFO Client 22284] : You have entered Overseer's Tower.";
    const LEVEL: &str = "2025/11/26 19:10:00 617500000 cff945bb [INFO Client 22284] : Alef_Slayer (Slayer) is now level 46";

    #[test]
    fn follows_area_zone_and_level() {
        let mut s = LogState::default();
        assert!(apply(&mut s, AREA));
        assert!(apply(&mut s, ENTER));
        assert!(apply(&mut s, LEVEL));
        assert_eq!(s.area_level, Some(45));
        assert_eq!(s.act, Some(5));
        assert_eq!(s.zone.as_deref(), Some("Overseer's Tower"));
        assert_eq!(s.character.as_deref(), Some("Alef_Slayer"));
        assert_eq!(s.class_name.as_deref(), Some("Slayer"));
        assert_eq!(s.level, Some(46));
        assert!(!apply(&mut s, ENTER), "a repeated line changes nothing");
    }

    #[test]
    fn chat_cannot_fake_a_level() {
        let mut s = LogState::default();
        apply(&mut s, "2025/11/26 19:10:00 1 a [INFO Client 1] #Someone: Bob (Witch) is now level 99");
        assert_eq!(s.level, None);
    }

    #[test]
    fn hideouts_and_maps_keep_or_end_the_act() {
        let mut s = LogState::default();
        apply(&mut s, r#"x [DEBUG Client 1] Generating level 60 area "2_9_1" with seed 1"#);
        assert_eq!(s.act, Some(9));
        apply(&mut s, r#"x [DEBUG Client 1] Generating level 20 area "HideoutBaths" with seed 1"#);
        assert_eq!((s.act, s.endgame), (Some(9), false));
        apply(&mut s, r#"x [DEBUG Client 1] Generating level 75 area "MapWorldsStrand" with seed 1"#);
        assert!(s.endgame);
    }

    #[test]
    fn acts_from_area_ids() {
        assert_eq!(act_of("1_1_1"), Some(1));
        assert_eq!(act_of("2_10_town"), Some(10));
        assert_eq!(act_of("G3_town"), Some(3));
        assert_eq!(act_of("C_G2_1"), Some(2));
        assert_eq!(act_of("HideoutBaths"), None);
        assert_eq!(act_of("MapWorldsStrand"), None);
    }

    #[test]
    fn reads_whole_lines_and_restarts_after_truncation() {
        let path = std::env::temp_dir().join(format!("pob-gamelog-{}.txt", std::process::id()));
        std::fs::write(&path, format!("{AREA}\n{ENTER}\npartial")).unwrap();
        let mut s = LogState::default();
        let (offset, changed) = read_from(&path, 0, &mut s).unwrap();
        assert!(changed);
        assert_eq!(offset as usize, AREA.len() + ENTER.len() + 2);
        std::fs::write(&path, format!("{LEVEL}\n")).unwrap();
        let (_, changed) = read_from(&path, offset, &mut s).unwrap();
        assert!(changed && s.level == Some(46), "a shorter file is read again from the start");
        let _ = std::fs::remove_file(path);
    }
}
