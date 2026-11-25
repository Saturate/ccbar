use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

/// `$XDG_CACHE_HOME/ccbar`, else `~/.cache/ccbar`.
pub fn cache_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    // XDG says an empty value counts as unset; without this the cache would
    // land in a relative `ccbar/` inside whatever project the statusline runs in.
    match std::env::var("XDG_CACHE_HOME") {
        Ok(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from(home).join(".cache"),
    }
    .join("ccbar")
}

/// Write via temp + rename so a concurrent reader never sees a torn file.
/// The temp name carries the pid: every tab renders about once a second, and a
/// shared temp name lets one writer truncate the file another just renamed.
pub fn write_atomic<T: Serialize>(path: &Path, value: &T) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let Ok(json) = serde_json::to_string(value) else {
        return;
    };
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return;
    };
    let tmp = path.with_file_name(format!(".{name}.{}.tmp", std::process::id()));
    if fs::write(&tmp, json).is_ok() && fs::rename(&tmp, path).is_err() {
        let _ = fs::remove_file(&tmp);
    }
}

/// FNV-1a: a stable hash for cache file names. std's hasher may change
/// between Rust releases, which would orphan cache files on upgrade.
pub fn stable_hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_round_trips_and_leaves_no_temp() {
        let dir = std::env::temp_dir().join(format!("ccbar-fsutil-{}", std::process::id()));
        let path = dir.join("x.json");
        write_atomic(&path, &vec![1, 2, 3]);
        let back: Vec<i32> = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(back, vec![1, 2, 3]);
        let leftovers = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftovers, 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn hash_is_stable() {
        assert_eq!(stable_hash(""), 0xcbf29ce484222325);
        assert_eq!(stable_hash("/x"), stable_hash("/x"));
        assert_ne!(stable_hash("/x"), stable_hash("/y"));
    }
}
