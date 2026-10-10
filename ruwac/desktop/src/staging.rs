//! Per-client private PNG namespace, with an advisory lock for crash reclamation.
use std::{
    fs::{self, File, OpenOptions},
    io,
    os::unix::{
        fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
        io::AsRawFd,
    },
    path::{Path, PathBuf},
};
pub struct Stage {
    pub path: PathBuf,
    _lock: File,
}
fn owned_dir(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| {
        m.is_dir()
            && !m.file_type().is_symlink()
            && m.uid() == unsafe { libc::geteuid() }
            && m.mode() & 0o777 == 0o700
    })
}
fn acquire(path: &Path) -> io::Result<File> {
    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path.join(".owner"))?;
    let m = f.metadata()?;
    if !m.is_file() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o777 != 0o600 {
        return Err(io::Error::other("stage marker rejected"));
    }
    if unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(io::Error::other("stage is in use"));
    }
    Ok(f)
}
fn reclaim(path: &Path) -> io::Result<()> {
    if !owned_dir(path) {
        return Err(io::Error::other("stage ownership rejected"));
    }
    let entries = fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
    // Unknown files or symlinks preserve the namespace rather than broad deletion.
    for e in &entries {
        let m = fs::symlink_metadata(e.path())?;
        let name = e.file_name().to_string_lossy().to_string();
        if !m.is_file()
            || m.file_type().is_symlink()
            || m.uid() != unsafe { libc::geteuid() }
            || m.mode() & 0o777 != 0o600
            || !(name == ".owner" || name.ends_with(".png"))
        {
            return Err(io::Error::other("unknown stage entry preserved"));
        }
    }
    for e in entries {
        fs::remove_file(e.path())?;
    }
    fs::remove_dir(path)
}
impl Stage {
    pub fn new(base: &Path) -> io::Result<Self> {
        if !owned_dir(base) {
            return Err(io::Error::other("private staging base required"));
        }
        for entry in fs::read_dir(base)?.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("ruwac-desktop-stage-") && owned_dir(&entry.path()) {
                if let Ok(_lock) = acquire(&entry.path()) {
                    let _ = reclaim(&entry.path());
                }
            }
        }
        let path = base.join(format!(
            "ruwac-desktop-stage-{}",
            ruwac_protocol::id("image")
        ));
        fs::DirBuilder::new().mode(0o700).create(&path)?;
        let lock = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path.join(".owner"))?;
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { path, _lock: lock })
    }
}
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = reclaim(&self.path);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn live_stage_and_crash_cleanup() {
        let base = std::env::temp_dir().join(ruwac_protocol::id("stage"));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        let live = Stage::new(&base).unwrap();
        let old = Stage::new(&base).unwrap();
        let file = old.path.join("receipt.png");
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&file)
            .unwrap();
        let old_path = old.path.clone(); // Forkless crash simulation: unlock, then forget the cleanup guard.
        unsafe {
            libc::flock(old._lock.as_raw_fd(), libc::LOCK_UN);
        }
        std::mem::forget(old);
        let next = Stage::new(&base).unwrap();
        assert!(!old_path.exists());
        assert!(live.path.exists());
        drop(next);
        drop(live);
        fs::remove_dir(base).unwrap();
    }
}
