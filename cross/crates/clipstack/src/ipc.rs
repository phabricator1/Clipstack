//! Hand-off from the one-shot CLI (`clipstack pop`, `pop-oldest`, `clear`) to the running
//! tray app.
//!
//! The CLI must not touch the clipboard or `history.json` itself while the tray is running:
//! the tray would see the CLI's clipboard write as a fresh copy and push the item straight
//! back onto the stack, and both processes would race over the history file. Instead the
//! CLI drops a small file into `<config dir>/commands/`; the tray drains that directory on
//! every tick (50 ms), performs the action in-process and deletes the file. The CLI waits
//! for the file to disappear, and only if nobody consumes it (no tray running) does it fall
//! back to acting on the clipboard and the store directly.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::config::Config;

const EXT: &str = "cmd";

pub fn dir() -> PathBuf {
    Config::dir().join("commands")
}

/// Written by the CLI. Returns `Ok(true)` when a running tray consumed the command within
/// `timeout`, `Ok(false)` when nothing picked it up (the file is removed again so a tray
/// started later does not replay it).
pub fn send(cmd: &str, timeout: Duration) -> io::Result<bool> {
    let dir = dir();
    fs::create_dir_all(&dir)?;
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let name = format!("{nanos:020}-{}.{EXT}", std::process::id());
    let path = dir.join(&name);
    // Write to a temp name and rename so the tray never reads a half-written file.
    let tmp = dir.join(format!(".{name}.tmp"));
    fs::write(&tmp, cmd)?;
    fs::rename(&tmp, &path)?;

    let deadline = Instant::now() + timeout;
    loop {
        if !path.exists() {
            return Ok(true);
        }
        if Instant::now() >= deadline {
            let _ = fs::remove_file(&path);
            return Ok(false);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Called by the tray. Returns pending commands in arrival order and removes their files.
pub fn drain() -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir()) else { return Vec::new() };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some(EXT))
        .collect();
    files.sort();
    files
        .into_iter()
        .filter_map(|p| {
            let cmd = fs::read_to_string(&p).ok();
            let _ = fs::remove_file(&p);
            cmd.map(|c| c.trim().to_string()).filter(|c| !c.is_empty())
        })
        .collect()
}

/// Called once when the tray starts: commands left over from a previous run (e.g. the
/// tray crashed while one was queued) must not be executed out of the blue.
pub fn discard_stale() {
    let n = drain().len();
    if n > 0 {
        log::info!("discarded {n} stale command file(s)");
    }
}
