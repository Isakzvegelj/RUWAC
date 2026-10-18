//! An implicit upload reuses the terminal's authenticated OpenSSH transport.
//! No alias/configuration lookup may create a replacement connection.
use std::{
    fs, io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt},
    path::{Path, PathBuf},
};

const MAX_CONTROLS: usize = 512;

fn private_dir(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o777 != 0o700
    {
        return Err(io::Error::other("Unsafe SSH transport directory"));
    }
    Ok(())
}

fn runtime() -> PathBuf {
    // A short kernel-managed path also works when the desktop's private
    // XDG_RUNTIME_DIR is too long for an OpenSSH Unix control socket.
    PathBuf::from(format!("/run/user/{}", unsafe { libc::geteuid() }))
}

fn folder() -> io::Result<PathBuf> {
    let root = runtime();
    private_dir(&root)?;
    let folder = root.join("ruwac-ssh");
    match fs::DirBuilder::new().mode(0o700).create(&folder) {
        Ok(()) => (),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
        Err(e) => return Err(e),
    }
    private_dir(&folder)?;
    Ok(folder)
}

pub fn valid(token: &str) -> bool {
    ruwac_protocol::valid_id(token, "transport")
}

fn path(token: &str) -> io::Result<PathBuf> {
    if !valid(token) {
        return Err(io::Error::other("Invalid SSH transport identity"));
    }
    let path = folder()?.join(token);
    if path.as_os_str().as_encoded_bytes().len() > 99 {
        return Err(io::Error::other("SSH transport path is too long"));
    }
    Ok(path)
}

/// Call before adding the optional SSH_ASKPASS environment wrapper. OpenSSH
/// command-line options come first so saved/user config cannot override them.
pub fn prepare(argv: &[String]) -> io::Result<Vec<String>> {
    if argv.first().map(String::as_str) != Some("ssh") {
        return Err(io::Error::other("Expected an SSH launch"));
    }
    let folder = folder()?;
    if fs::read_dir(&folder)?.take(MAX_CONTROLS).count() >= MAX_CONTROLS {
        return Err(io::Error::other("SSH transport capacity reached"));
    }
    let token = ruwac_protocol::id("transport");
    let path = path(&token)?;
    if fs::symlink_metadata(&path).is_ok() {
        return Err(io::Error::other("SSH transport identity already exists"));
    }
    let mut planned = vec![
        "ssh".into(),
        "-o".into(),
        "ControlMaster=yes".into(),
        "-o".into(),
        "ControlPersist=no".into(),
        "-o".into(),
        format!("ControlPath={}", path.display()),
    ];
    planned.extend_from_slice(&argv[1..]);
    Ok(planned)
}

/// Store only the generated identity, never resolved addresses or credentials.
pub fn identity(argv: &[String]) -> Option<String> {
    let parent = runtime().join("ruwac-ssh");
    argv.windows(2).find_map(|pair| {
        if pair[0] != "-o" {
            return None;
        }
        let path = Path::new(pair[1].strip_prefix("ControlPath=")?);
        if path.parent()? != parent {
            return None;
        }
        let token = path.file_name()?.to_str()?;
        valid(token).then(|| token.into())
    })
}

/// The only permitted network operation is opening a channel on this socket.
/// Missing/closed masters fail through /bin/false; current user/system SSH
/// config is ignored, including aliases, ProxyJump and remote commands.
pub fn client(token: &str, pid: u32) -> io::Result<Vec<String>> {
    let path = path(token)?;
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.file_type().is_socket() || metadata.uid() != unsafe { libc::geteuid() } {
        return Err(io::Error::other("Original SSH transport is unavailable"));
    }
    verify_peer(&path, pid)?;
    Ok([
        "ssh",
        "-F",
        "/dev/null",
        "-S",
        path.to_str()
            .ok_or_else(|| io::Error::other("Invalid SSH socket path"))?,
        "-o",
        "ControlMaster=no",
        "-o",
        "ProxyCommand=/bin/false",
        "-o",
        "BatchMode=yes",
        "-o",
        "ForwardAgent=no",
        "-o",
        "ClearAllForwardings=yes",
        "-T",
        "--",
        "ruwac-bound-transport",
    ]
    .into_iter()
    .map(String::from)
    .collect())
}

/// Nonblocking local connect avoids an unbounded wait on a full accept queue.
/// The kernel identifies the SSH process owning the listener, independently of
/// the pathname. Inspecting the peer never sends an SSH command or image data.
fn verify_peer(path: &Path, expected_pid: u32) -> io::Result<()> {
    if expected_pid == 0 {
        return Err(io::Error::other("Original SSH process is unavailable"));
    }
    let raw = unsafe {
        libc::socket(
            libc::AF_UNIX,
            libc::SOCK_STREAM | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
            0,
        )
    };
    if raw < 0 {
        return Err(io::Error::last_os_error());
    }
    let fd = unsafe { OwnedFd::from_raw_fd(raw) };
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    address.sun_family = libc::AF_UNIX as libc::sa_family_t;
    let bytes = path.as_os_str().as_encoded_bytes();
    if bytes.len() >= address.sun_path.len() {
        return Err(io::Error::other("SSH transport path is too long"));
    }
    for (dst, src) in address.sun_path.iter_mut().zip(bytes) {
        *dst = *src as libc::c_char;
    }
    let length =
        (std::mem::offset_of!(libc::sockaddr_un, sun_path) + bytes.len() + 1) as libc::socklen_t;
    if unsafe {
        libc::connect(
            fd.as_raw_fd(),
            (&address as *const libc::sockaddr_un).cast(),
            length,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    let mut peer: libc::ucred = unsafe { std::mem::zeroed() };
    let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    if unsafe {
        libc::getsockopt(
            fd.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut peer as *mut libc::ucred).cast(),
            &mut length,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    if length as usize != std::mem::size_of::<libc::ucred>()
        || peer.uid != unsafe { libc::geteuid() }
        || u32::try_from(peer.pid).ok() != Some(expected_pid)
    {
        return Err(io::Error::other("Original SSH transport process changed"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_generated_transport_identities_are_accepted() {
        assert!(valid(&ruwac_protocol::id("transport")));
        for token in ["", "../socket", "/tmp/socket", "transport_x", "pane_123"] {
            assert!(!valid(token));
        }
    }
    #[test]
    fn external_control_paths_are_not_catalog_identities() {
        assert_eq!(
            identity(&["ssh".into(), "-o".into(), "ControlPath=/tmp/socket".into()]),
            None
        );
    }
}
