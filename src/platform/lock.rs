//! Single-instance lock for macOS and Linux: an OS file lock, which the OS
//! drops when the holder exits or crashes, so a stale file never blocks.

use std::fs::{File, OpenOptions};
use std::path::Path;

pub struct Lock(#[allow(dead_code)] File);

impl Lock {
    /// None when another process holds the lock (or the file can't be opened).
    pub fn claim(path: &Path) -> Option<Lock> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let f = OpenOptions::new().create(true).truncate(false).write(true).open(path).ok()?;
        f.try_lock().ok()?;
        Some(Lock(f))
    }
}
