//! Only view identities are saved; terminal output and secrets stay outside workspace metadata.
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workspace {
    pub schema: u32,
    pub socket: PathBuf,
    pub sessions: Vec<String>,
    pub selected: Option<String>,
}
fn directory() -> io::Result<PathBuf> {
    let p = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")))
        .ok_or_else(|| io::Error::other("configuration directory unavailable"))?
        .join("RUWAC");
    if !p.exists() {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&p)?;
    }
    let m = fs::symlink_metadata(&p)?;
    if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o777 != 0o700 {
        return Err(io::Error::other("unsafe workspace directory"));
    }
    Ok(p)
}
pub fn load(socket: &Path) -> io::Result<Workspace> {
    let p = directory()?.join("workspace.json");
    let mut file = match fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(p)
    {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Workspace::default()),
        Err(e) => return Err(e),
    };
    let m = file.metadata()?;
    if !m.is_file()
        || m.uid() != unsafe { libc::geteuid() }
        || m.mode() & 0o777 != 0o600
        || m.len() > 65536
    {
        return Err(io::Error::other("unsafe workspace file"));
    }
    let mut data = Vec::new();
    Read::by_ref(&mut file).take(65537).read_to_end(&mut data)?;
    if data.len() > 65536 {
        return Err(io::Error::other("workspace exceeds limit"));
    }
    let w: Workspace = serde_json::from_slice(&data)?;
    if w.schema != 1
        || w.sessions.len() > 64
        || w.sessions
            .iter()
            .any(|s| !ruwac_protocol::valid_id(s, "session"))
        || w.selected.as_ref().is_some_and(|s| !w.sessions.contains(s))
    {
        return Err(io::Error::other("invalid workspace schema or identities"));
    }
    if w.socket != socket {
        return Ok(Workspace::default());
    }
    Ok(w)
}
pub fn save(w: &Workspace) -> io::Result<()> {
    let dir = directory()?;
    let path = dir.join("workspace.json");
    // Preserve corrupt/unrecognized metadata instead of silently overwriting it.
    let _ = load(&w.socket)?;
    let temp = dir.join(ruwac_protocol::id("workspace-tmp"));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&temp)?;
        file.write_all(&serde_json::to_vec_pretty(w)?)?;
        file.sync_all()?;
        fs::rename(&temp, &path)?;
        fs::File::open(&dir)?.sync_all()
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}
