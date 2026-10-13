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
const RETENTION: &str = ".retain-until";
const MAX_STAGE_BYTES: u64 = 256 * 1024 * 1024;
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
/// Reserve room for both the incoming source file and its validated final copy.
pub fn available(path: &Path) -> io::Result<usize> {
    if !owned_dir(path) {
        return Err(io::Error::other("Private image namespace required"));
    }
    let mut bytes = 0u64;
    let mut images = 0;
    for (index, entry) in fs::read_dir(path)?.enumerate() {
        if index >= 256 {
            return Err(io::Error::other("Image namespace entry limit reached"));
        }
        let entry = entry?;
        let name = entry.file_name();
        if name == ".owner" || name == RETENTION {
            continue;
        }
        let m = fs::symlink_metadata(entry.path())?;
        if !m.is_file() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o777 != 0o600 {
            return Err(io::Error::other("Unrecognized staging entry preserved"));
        }
        images += 1;
        bytes = bytes.saturating_add(m.len());
    }
    if images >= 128 || bytes >= MAX_STAGE_BYTES {
        return Err(io::Error::other(
            "Image staging quota reached; discard old drafts before staging more",
        ));
    }
    Ok(((MAX_STAGE_BYTES - bytes) / 2).min(ruwac_clipboard::MAX_IMAGE_BYTES as u64) as usize)
}
/// Keep attachment files across frontend close: Codex reads them when the user submits.
/// A later frontend startup reclaims the owned namespace after 24 hours.
pub fn retain(path: &Path) -> io::Result<()> {
    use std::io::Write;
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("Image namespace missing"))?;
    if !owned_dir(parent) {
        return Err(io::Error::other("Image namespace ownership rejected"));
    }
    let mut marker = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(parent.join(RETENTION))?;
    let m = marker.metadata()?;
    if !m.is_file()
        || m.uid() != unsafe { libc::geteuid() }
        || m.mode() & 0o777 != 0o600
        || m.nlink() != 1
    {
        return Err(io::Error::other("Retention marker rejected"));
    }
    marker.set_len(0)?;
    write!(marker, "{}", now() + 24 * 60 * 60)?;
    marker.sync_all()?;
    Ok(())
}
/// Expire only owned PNGs in this frontend's namespace. Live transfer files are preserved.
pub fn expire(path: &Path) -> io::Result<usize> {
    use std::io::Read;
    if !owned_dir(path) {
        return Err(io::Error::other("Image namespace ownership rejected"));
    }
    let marker = path.join(RETENTION);
    if !marker.exists() && !marker.is_symlink() {
        return Ok(0);
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(&marker)?;
    let m = file.metadata()?;
    if !m.is_file()
        || m.uid() != unsafe { libc::geteuid() }
        || m.mode() & 0o777 != 0o600
        || m.len() > 32
        || m.nlink() != 1
    {
        return Err(io::Error::other("Retention marker rejected"));
    }
    let mut value = String::new();
    file.take(33).read_to_string(&mut value)?;
    let expires = value
        .parse::<u64>()
        .map_err(|_| io::Error::other("Invalid retention marker preserved"))?;
    if expires > now() {
        return Ok(0);
    }
    let mut expired = 0;
    for (index, entry) in fs::read_dir(path)?.enumerate() {
        if index >= 256 {
            return Err(io::Error::other("Image namespace entry limit reached"));
        }
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("transfer-") || !name.ends_with(".png") {
            continue;
        }
        let m = fs::symlink_metadata(entry.path())?;
        if !m.is_file()
            || m.uid() != unsafe { libc::geteuid() }
            || m.mode() & 0o777 != 0o600
            || m.nlink() != 1
        {
            return Err(io::Error::other("Unknown staging image preserved"));
        }
        fs::remove_file(entry.path())?;
        expired += 1;
    }
    fs::remove_file(marker)?;
    Ok(expired)
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
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
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
    let marker = path.join(RETENTION);
    if marker.exists() || marker.is_symlink() {
        use std::io::Read;
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&marker)?;
        let m = file.metadata()?;
        if !m.is_file()
            || m.uid() != unsafe { libc::geteuid() }
            || m.mode() & 0o777 != 0o600
            || m.len() > 32
            || m.nlink() != 1
        {
            return Err(io::Error::other("Retention marker preserved"));
        }
        let mut value = String::new();
        file.take(33).read_to_string(&mut value)?;
        let expires = value
            .parse::<u64>()
            .map_err(|_| io::Error::other("Invalid retention marker preserved"))?;
        if expires > now() {
            return Err(io::Error::other(
                "Pending agent attachment retained until expiry",
            ));
        }
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
            || !(name == ".owner" || name == RETENTION || name.ends_with(".png"))
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
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
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
mod attachment_tests {
    use super::*;
    #[test]
    fn source_and_final_copy_share_the_peak_byte_quota() {
        let base = std::env::temp_dir().join(ruwac_protocol::id("stage-test"));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        let stage = Stage::new(&base).unwrap();
        let existing = stage.path.join("existing.png");
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&existing)
            .unwrap();
        file.set_len(192 * 1024 * 1024).unwrap();
        let cap = available(&stage.path).unwrap();
        assert_eq!(cap, 32 * 1024 * 1024);
        for name in ["transfer-pending.png", "final.png"] {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(stage.path.join(name))
                .unwrap()
                .set_len(cap as u64)
                .unwrap();
        }
        assert!(available(&stage.path).is_err());
        drop(file);
        drop(stage);
        fs::remove_dir(base).unwrap();
    }
    #[test]
    fn draft_survives_frontend_close_and_expired_namespace_is_reclaimed() {
        let base = std::env::temp_dir().join(ruwac_protocol::id("stage-test"));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        let stage = Stage::new(&base).unwrap();
        let path = stage.path.join("retained.png");
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .unwrap();
        retain(&path).unwrap();
        assert_eq!(
            available(&stage.path).unwrap(),
            ruwac_clipboard::MAX_IMAGE_BYTES
        );
        let namespace = stage.path.clone();
        drop(stage);
        assert!(path.exists());
        fs::write(namespace.join(RETENTION), b"0").unwrap();
        let next = Stage::new(&base).unwrap();
        assert!(!namespace.exists());
        drop(next);
        fs::remove_dir(base).unwrap();
    }
    #[test]
    fn unknown_entry_and_retention_symlink_are_preserved() {
        let base = std::env::temp_dir().join(ruwac_protocol::id("stage-test"));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        let stage = Stage::new(&base).unwrap();
        let path = stage.path.join("retained.png");
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .unwrap();
        std::os::unix::fs::symlink(&path, stage.path.join(RETENTION)).unwrap();
        assert!(retain(&path).is_err());
        assert!(reclaim(&stage.path).is_err());
        fs::remove_file(stage.path.join(RETENTION)).unwrap();
        drop(stage);
        fs::remove_dir(base).unwrap();
    }
    #[test]
    fn active_namespace_expiry_preserves_live_transfer() {
        let base = std::env::temp_dir().join(ruwac_protocol::id("stage-test"));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        let stage = Stage::new(&base).unwrap();
        let image = stage.path.join("expired.png");
        let transfer = stage.path.join("transfer-pending.png");
        for p in [&image, &transfer] {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(p)
                .unwrap();
        }
        retain(&image).unwrap();
        assert_eq!(expire(&stage.path).unwrap(), 0);
        fs::write(stage.path.join(RETENTION), b"0").unwrap();
        assert_eq!(expire(&stage.path).unwrap(), 1);
        assert!(!image.exists());
        assert!(transfer.exists());
        drop(stage);
        fs::remove_dir(base).unwrap();
    }
    #[test]
    fn retention_fifo_refuses_without_waiting_for_a_peer() {
        use std::os::unix::ffi::OsStrExt;
        let base = std::env::temp_dir().join(ruwac_protocol::id("stage-test"));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        let stage = Stage::new(&base).unwrap();
        let image = stage.path.join("owned.png");
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&image)
            .unwrap();
        let marker = stage.path.join(RETENTION);
        let name = std::ffi::CString::new(marker.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        let start = std::time::Instant::now();
        assert!(retain(&image).is_err());
        assert!(expire(&stage.path).is_err());
        assert!(reclaim(&stage.path).is_err());
        assert!(start.elapsed() < std::time::Duration::from_secs(1));
        assert!(image.exists());
        fs::remove_file(marker).unwrap();
        drop(stage);
        fs::remove_dir(base).unwrap();
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
