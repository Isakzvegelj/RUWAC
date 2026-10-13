//! Private named snapshots. Automation supplies IDs, never paths or workspace payloads.
use crate::workspace::{self, Workspace};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    time::Instant,
};
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub id: String,
    pub name: String,
    pub workspace: Workspace,
}
pub fn folder(socket: &Path) -> io::Result<PathBuf> {
    let path = workspace::directory()?.join("snapshots");
    if !path.exists() {
        fs::DirBuilder::new().mode(0o700).create(&path)?;
    }
    let m = fs::symlink_metadata(&path)?;
    if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o777 != 0o700 {
        return Err(io::Error::other("Unsafe snapshot directory"));
    }
    let own = path.join(workspace::identity(socket));
    if !own.exists() {
        fs::DirBuilder::new().mode(0o700).create(&own)?;
    }
    let m = fs::symlink_metadata(&own)?;
    if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o777 != 0o700 {
        return Err(io::Error::other("Unsafe daemon snapshot directory"));
    }
    Ok(own)
}
fn valid(snapshot: &Snapshot) -> io::Result<()> {
    if !ruwac_protocol::valid_id(&snapshot.id, "workspace")
        || snapshot.name.is_empty()
        || snapshot.name.len() > 128
        || snapshot.name.chars().any(char::is_control)
    {
        return Err(io::Error::other("Invalid snapshot identity or name"));
    }
    workspace::validate(&snapshot.workspace)
}
pub fn read(folder: &Path, id: &str, socket: &Path) -> io::Result<Snapshot> {
    if !ruwac_protocol::valid_id(id, "workspace") {
        return Err(io::Error::other("Explicit snapshot ID required"));
    }
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(folder.join(format!("{id}.json")))?;
    let m = file.metadata()?;
    if !m.is_file()
        || m.uid() != unsafe { libc::geteuid() }
        || m.mode() & 0o777 != 0o600
        || m.nlink() != 1
        || m.len() > 1048576
    {
        return Err(io::Error::other("Unsafe snapshot file"));
    }
    let mut bytes = Vec::new();
    file.take(1048577).read_to_end(&mut bytes)?;
    if bytes.len() > 1048576 {
        return Err(io::Error::other("Snapshot exceeds cap"));
    }
    let value: Snapshot = serde_json::from_slice(&bytes)?;
    valid(&value)?;
    if value.id != id || value.workspace.socket != socket {
        return Err(io::Error::other(
            "Snapshot identity or daemon binding differs",
        ));
    }
    Ok(value)
}
pub fn list(folder: &Path, socket: &Path) -> io::Result<Value> {
    let mut rows = Vec::new();
    for (index, entry) in fs::read_dir(folder)?.enumerate() {
        if index >= 128 {
            return Err(io::Error::other("Snapshot directory entry cap exceeded"));
        }
        let entry = entry?;
        let name = entry.file_name();
        let Some(id) = name.to_str().and_then(|s| s.strip_suffix(".json")) else {
            continue;
        };
        if !ruwac_protocol::valid_id(id, "workspace") {
            continue;
        }
        let value = read(folder, id, socket)?;
        rows.push(summary(&value));
        if rows.len() > 64 {
            return Err(io::Error::other("Saved workspace cap exceeded"));
        }
    }
    rows.sort_by_key(|v| v["id"].as_str().unwrap_or("").to_owned());
    Ok(json!({"saved_workspaces":rows}))
}
pub fn summary(s: &Snapshot) -> Value {
    json!({"id":s.id,"name":s.name,"tabs":s.workspace.tabs.len(),"terminal_sessions":s.workspace.sessions.len()})
}
pub fn save(
    folder: &Path,
    snapshot: &Snapshot,
    expires: Instant,
    permission: &std::sync::atomic::AtomicBool,
) -> io::Result<Value> {
    use std::os::fd::AsRawFd;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(folder.join(".snapshots-lock"))?;
    let m = lock.metadata()?;
    if !m.is_file()
        || m.uid() != unsafe { libc::geteuid() }
        || m.mode() & 0o777 != 0o600
        || m.nlink() != 1
    {
        return Err(io::Error::other("Unsafe snapshot lock"));
    }
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "Snapshot save already in progress",
        ));
    }
    valid(snapshot)?;
    if list(folder, &snapshot.workspace.socket)?["saved_workspaces"]
        .as_array()
        .is_none_or(|rows| rows.len() >= 64)
    {
        return Err(io::Error::other("Saved workspace cap reached"));
    }
    let bytes = serde_json::to_vec_pretty(snapshot)?;
    if bytes.len() > 1048576 {
        return Err(io::Error::other("Snapshot exceeds cap"));
    }
    let temp = folder.join(ruwac_protocol::id("snapshot-tmp"));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        if !permission.load(std::sync::atomic::Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Workspace permission revoked",
            ));
        }
        if Instant::now() >= expires {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Snapshot request expired before publication",
            ));
        }
        fs::hard_link(&temp, folder.join(format!("{}.json", snapshot.id)))?;
        fs::remove_file(&temp)?;
        fs::File::open(folder)?.sync_all()?;
        Ok(summary(snapshot))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (PathBuf, Snapshot) {
        let path = std::env::temp_dir().join(ruwac_protocol::id("snapshot-test"));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        let snapshot = Snapshot {
            id: ruwac_protocol::id("workspace"),
            name: "Private layout".into(),
            workspace: Workspace {
                schema: 3,
                socket: "private.sock".into(),
                ..Workspace::default()
            },
        };
        (path, snapshot)
    }
    #[test]
    fn atomic_private_snapshot_and_daemon_binding() {
        let (path, snapshot) = fixture();
        let permission = std::sync::atomic::AtomicBool::new(true);
        save(
            &path,
            &snapshot,
            Instant::now() + std::time::Duration::from_secs(2),
            &permission,
        )
        .unwrap();
        assert_eq!(
            read(&path, &snapshot.id, Path::new("private.sock"))
                .unwrap()
                .name,
            "Private layout"
        );
        assert!(read(&path, &snapshot.id, Path::new("another.sock")).is_err());
        assert_eq!(
            fs::metadata(path.join(format!("{}.json", snapshot.id)))
                .unwrap()
                .mode()
                & 0o777,
            0o600
        );
        fs::remove_dir_all(path).unwrap();
    }
    #[test]
    fn expired_or_revoked_save_never_publishes() {
        let (path, snapshot) = fixture();
        let permission = std::sync::atomic::AtomicBool::new(true);
        assert!(save(&path, &snapshot, Instant::now(), &permission).is_err());
        permission.store(false, std::sync::atomic::Ordering::Release);
        assert!(
            save(
                &path,
                &snapshot,
                Instant::now() + std::time::Duration::from_secs(2),
                &permission
            )
            .is_err()
        );
        assert!(!path.join(format!("{}.json", snapshot.id)).exists());
        assert!(!fs::read_dir(&path).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("snapshot-tmp")
        }));
        fs::remove_dir_all(path).unwrap();
    }
    #[test]
    fn symlink_fifo_and_hardlink_snapshots_are_refused() {
        use std::os::unix::ffi::OsStrExt;
        let (path, snapshot) = fixture();
        let file = path.join(format!("{}.json", snapshot.id));
        std::os::unix::fs::symlink("not-owned", &file).unwrap();
        assert!(read(&path, &snapshot.id, Path::new("private.sock")).is_err());
        fs::remove_file(&file).unwrap();
        let name = std::ffi::CString::new(file.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        let start = Instant::now();
        assert!(read(&path, &snapshot.id, Path::new("private.sock")).is_err());
        assert!(start.elapsed() < std::time::Duration::from_secs(1));
        fs::remove_file(&file).unwrap();
        let permission = std::sync::atomic::AtomicBool::new(true);
        save(
            &path,
            &snapshot,
            Instant::now() + std::time::Duration::from_secs(2),
            &permission,
        )
        .unwrap();
        fs::hard_link(&file, path.join("alias")).unwrap();
        assert!(read(&path, &snapshot.id, Path::new("private.sock")).is_err());
        fs::remove_dir_all(path).unwrap();
    }
}
