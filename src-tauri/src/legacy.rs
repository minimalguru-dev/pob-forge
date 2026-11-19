//! One-time carry-over of settings, history and saved keys from the identities
//! earlier builds of this app ran under. Copies, never moves: the old folders
//! stay where they were, so a failed copy loses nothing.

use std::fs;
use std::path::Path;

/// Identifiers earlier builds used, newest first. The first one found is copied.
pub(crate) const OLD_IDENTIFIERS: [&str; 2] = ["dev.pobredux.desktop", "dev.minimalguru.pob-progression-preview"];

/// DPAPI entropy the Windows key files were encrypted with before the rename.
#[cfg(windows)]
pub(crate) const OLD_KEY_ENTROPY: &str = "dev.pobredux.desktop";

/// WebView caches that the browser rebuilds on its own; not worth copying.
const SKIP: [&str; 4] = ["Cache", "Code Cache", "GPUCache", "Crashpad"];

/// Runs before any window exists, so the webview profile (and its localStorage)
/// is copied before the webview opens it.
pub(crate) fn migrate(identifier: &str) {
    if OLD_IDENTIFIERS.contains(&identifier) {
        return;
    }
    let mut bases: Vec<_> = [dirs::config_dir(), dirs::data_dir(), dirs::data_local_dir(), dirs::cache_dir()].into_iter().flatten().collect();
    bases.sort();
    bases.dedup();
    for base in bases {
        let new = base.join(identifier);
        if new.exists() {
            continue;
        }
        let Some(old) = OLD_IDENTIFIERS.iter().map(|id| base.join(id)).find(|p| p.is_dir()) else { continue };
        match copy_dir(&old, &new) {
            Ok(()) => log::info!("copied {} to {}", old.display(), new.display()),
            Err(e) => {
                log::warn!("could not copy {} to {}: {e}", old.display(), new.display());
                // A half-copied folder would hide the old one from the next attempt.
                let _ = fs::remove_dir_all(&new);
            }
        }
    }
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if SKIP.iter().any(|s| name == *s) {
            continue;
        }
        let kind = entry.file_type()?;
        let target = to.join(&name);
        if kind.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else if kind.is_file() {
            // Lock files belong to the process that held them.
            if name == "session.lock" || name == "LOCK" || name == "lockfile" {
                continue;
            }
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn copies_nested_files_and_skips_caches() {
        let root = std::env::temp_dir().join(format!("pob-forge-legacy-{}", std::process::id()));
        let from = root.join("old");
        std::fs::create_dir_all(from.join("EBWebView/Default/Cache")).unwrap();
        std::fs::create_dir_all(from.join("EBWebView/Default/Local Storage")).unwrap();
        std::fs::write(from.join("settings.json"), b"{}").unwrap();
        std::fs::write(from.join("session.lock"), b"1").unwrap();
        std::fs::write(from.join("EBWebView/Default/Cache/blob"), b"x").unwrap();
        std::fs::write(from.join("EBWebView/Default/Local Storage/leveldb"), b"kv").unwrap();
        super::copy_dir(&from, &root.join("new")).unwrap();
        assert_eq!(std::fs::read(root.join("new/settings.json")).unwrap(), b"{}");
        assert_eq!(std::fs::read(root.join("new/EBWebView/Default/Local Storage/leveldb")).unwrap(), b"kv");
        assert!(!root.join("new/session.lock").exists());
        assert!(!root.join("new/EBWebView/Default/Cache").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
