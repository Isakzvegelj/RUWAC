//! Explicit SSH rsync transport with a bounded literal file list and verified staging.
use crate::profiles::{Profile, endpoint};
use sha2::{Digest, Sha256};
use ssh2::{FileStat, Sftp};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    os::unix::{
        ffi::OsStrExt,
        fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
        process::CommandExt,
    },
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
const MAX_BYTES: u64 = 32 * 1024 * 1024 * 1024;
const MAX_ENTRIES: usize = 10_000;
#[derive(Clone)]
struct Entry {
    directory: bool,
    mode: u32,
    bytes: u64,
    hash: String,
}
type Manifest = BTreeMap<PathBuf, Entry>;
#[derive(serde::Serialize)]
pub struct Receipt {
    pub bytes: u64,
    pub files: usize,
    pub directories: usize,
    pub sha256: String,
}
fn error(_: impl std::fmt::Display) -> String {
    "Rsync file operation failed; destination was not replaced".into()
}
fn check(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) {
        Err("Rsync cancelled; destination preserved".into())
    } else {
        Ok(())
    }
}
fn valid(path: &Path) -> Result<(), String> {
    if path.to_str().is_none() {
        return Err("Non-UTF-8 transfer paths are unsupported".into());
    }
    if path.as_os_str().len() > 4096
        || path.as_os_str().as_bytes().contains(&0)
        || path.components().any(|c| matches!(c, Component::ParentDir))
    {
        return Err("Invalid transfer path".into());
    }
    Ok(())
}
fn node(root: &Path, relative: &Path) -> PathBuf {
    if relative.as_os_str().is_empty() {
        root.to_owned()
    } else {
        root.join(relative)
    }
}
fn sftp_error(stage: &str, e: ssh2::Error) -> String {
    format!("SFTP {stage} failed (code {:?})", e.code())
}
fn digest(
    mut reader: impl Read,
    cancel: &AtomicBool,
    deadline: Instant,
) -> Result<(u64, String), String> {
    let (mut count, mut hash, mut data) = (0u64, Sha256::new(), [0u8; 65536]);
    loop {
        check(cancel)?;
        if Instant::now() > deadline {
            return Err("Rsync verification timed out".into());
        }
        let n = reader.read(&mut data).map_err(error)?;
        if n == 0 {
            break;
        }
        count += n as u64;
        if count > MAX_BYTES {
            return Err("Rsync file exceeds 32 GiB".into());
        }
        hash.update(&data[..n]);
    }
    Ok((count, format!("{:x}", hash.finalize())))
}
fn scan(root: &Path, sftp: Option<&Sftp>, cancel: &AtomicBool) -> Result<Manifest, String> {
    fn visit(
        root: &Path,
        relative: PathBuf,
        sftp: Option<&Sftp>,
        cancel: &AtomicBool,
        out: &mut Manifest,
        total: &mut u64,
        depth: usize,
        deadline: Instant,
    ) -> Result<(), String> {
        check(cancel)?;
        if Instant::now() > deadline {
            return Err("Rsync tree preparation timed out".into());
        }
        if depth > 64 || out.len() >= MAX_ENTRIES {
            return Err("Rsync tree exceeds 10,000 entries or 64 directory levels".into());
        }
        let path = node(root, &relative);
        valid(&path)?;
        let (mode, size) = if let Some(s) = sftp {
            let m = s.lstat(&path).map_err(error)?;
            (
                m.perm.ok_or("Remote permissions unavailable")?,
                m.size.unwrap_or(0),
            )
        } else {
            let m = fs::symlink_metadata(&path).map_err(error)?;
            (m.mode(), m.len())
        };
        let directory = match mode & 0o170000 {
            0o040000 => true,
            0o100000 => false,
            _ => {
                return Err(
                    "Rsync refuses symlinks, devices and special files; source unchanged".into(),
                );
            }
        };
        if !directory && size > MAX_BYTES.saturating_sub(*total) {
            return Err("Rsync tree exceeds 32 GiB".into());
        }
        let (bytes, hash) = if directory {
            (0, String::new())
        } else if let Some(s) = sftp {
            digest(s.open(&path).map_err(error)?, cancel, deadline)?
        } else {
            digest(
                fs::OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                    .open(&path)
                    .map_err(error)?,
                cancel,
                deadline,
            )?
        };
        if !directory && bytes != size {
            return Err("Source changed while preparing transfer".into());
        }
        *total = total.checked_add(bytes).ok_or("Transfer size overflow")?;
        if *total > MAX_BYTES {
            return Err("Rsync tree exceeds 32 GiB".into());
        }
        out.insert(
            relative.clone(),
            Entry {
                directory,
                mode: mode & 0o777,
                bytes,
                hash,
            },
        );
        if directory {
            let names = if let Some(s) = sftp {
                remote_names(s, &path)?
            } else {
                fs::read_dir(&path)
                    .map_err(error)?
                    .take(MAX_ENTRIES + 1)
                    .map(|e| e.map(|e| e.file_name()).map_err(error))
                    .collect::<Result<Vec<_>, _>>()?
            };
            if names.len() > MAX_ENTRIES {
                return Err("Rsync directory exceeds 10,000 entries".into());
            }
            for name in names {
                if name == "." || name == ".." {
                    continue;
                }
                let part = Path::new(&name);
                if part.components().count() != 1
                    || !matches!(part.components().next(), Some(Component::Normal(_)))
                {
                    return Err("Invalid tree entry".into());
                }
                visit(
                    root,
                    relative.join(name),
                    sftp,
                    cancel,
                    out,
                    total,
                    depth + 1,
                    deadline,
                )?;
            }
        }
        Ok(())
    }
    let mut result = Manifest::new();
    visit(
        root,
        PathBuf::new(),
        sftp,
        cancel,
        &mut result,
        &mut 0,
        0,
        Instant::now() + Duration::from_secs(3600),
    )?;
    Ok(result)
}
// rsync's --rsh parser supports adjacent quoted strings, not backslash escapes.
fn rsh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
fn ssh_path(path: &Path) -> Result<String, String> {
    let value = path.to_str().ok_or("SSH key/trust path must be UTF-8")?;
    if value.chars().any(char::is_control) || value.contains('$') {
        return Err(
            "SSH key/trust paths with variable expansion syntax are unsupported; use SFTP".into(),
        );
    }
    Ok(value.replace('%', "%%"))
}
fn ssh_args(p: &Profile) -> Result<(Vec<String>, String), String> {
    p.validate()?;
    if p.protocol != "SSH" || !p.preferences.gateway.is_empty() {
        return Err(
            "Rsync requires a direct SSH endpoint; gateway routing is not configured".into(),
        );
    }
    let (host, port) = endpoint(&p.server, 22)?;
    if host.is_empty()
        || host.starts_with('-')
        || !host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-_:%".contains(&b))
    {
        return Err("Invalid SSH host".into());
    }
    let known = if p.preferences.known_hosts.is_empty() {
        PathBuf::from(std::env::var_os("HOME").ok_or("Home unavailable")?).join(".ssh/known_hosts")
    } else {
        PathBuf::from(&p.preferences.known_hosts)
    };
    let known = ssh_path(&known)?.replace('\\', "\\\\").replace('"', "\\\"");
    let mut args = vec![
        "-F".into(),
        "/dev/null".into(),
        "-T".into(),
        "-p".into(),
        port.to_string(),
        "-o".into(),
        "BatchMode=yes".into(),
        "-o".into(),
        "StrictHostKeyChecking=yes".into(),
        "-o".into(),
        "UpdateHostKeys=no".into(),
        "-o".into(),
        "GlobalKnownHostsFile=/dev/null".into(),
        "-o".into(),
        format!("UserKnownHostsFile=\"{known}\""),
        "-o".into(),
        "ConnectTimeout=8".into(),
        "-o".into(),
        "ServerAliveInterval=5".into(),
        "-o".into(),
        "ServerAliveCountMax=3".into(),
        "-o".into(),
        "LogLevel=ERROR".into(),
    ];
    if !p.preferences.identity_file.is_empty() {
        args.extend([
            "-i".into(),
            ssh_path(Path::new(&p.preferences.identity_file))?,
            "-o".into(),
            "IdentitiesOnly=yes".into(),
        ]);
    }
    if !p.username.is_empty() {
        args.extend(["-l".into(), p.username.clone()]);
    }
    Ok((args, host))
}
fn version(text: &[u8]) -> Result<(), String> {
    let text = std::str::from_utf8(text).map_err(error)?;
    let line = text
        .lines()
        .find_map(|l| l.strip_prefix("rsync  version "))
        .ok_or("Remote rsync was not detected")?;
    let mut digits = line.split_whitespace().next().unwrap_or("").split('.');
    let major = digits
        .next()
        .and_then(|v| v.parse::<u32>().ok())
        .ok_or("Unknown rsync version")?;
    let minor = digits
        .next()
        .and_then(|v| v.parse::<u32>().ok())
        .ok_or("Unknown rsync version")?;
    if (major, minor) < (3, 4) {
        return Err("Rsync 3.4 or newer is required on both endpoints".into());
    }
    Ok(())
}
fn progress_line(line: &[u8], progress: &AtomicU64) {
    if let Ok(text) = std::str::from_utf8(line) {
        let mut fields = text.split_whitespace();
        if let (Some(count), Some(percent)) = (fields.next(), fields.next()) {
            if percent.ends_with('%') {
                if let Ok(n) = count.replace(',', "").parse::<u64>() {
                    progress.fetch_max(n.min(MAX_BYTES), Ordering::Relaxed);
                }
            }
        }
    }
}
fn run(
    mut command: Command,
    cancel: &AtomicBool,
    progress: Arc<AtomicU64>,
    probe: bool,
) -> Result<Vec<u8>, String> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .env("LC_ALL", "C");
    for name in [
        "RSYNC_RSH",
        "RSYNC_OLD_ARGS",
        "RSYNC_PROTECT_ARGS",
        "RSYNC_PASSWORD",
        "RSYNC_PARTIAL_DIR",
        "RSYNC_PROXY",
        "RSYNC_CONNECT_PROG",
        "RSYNC_SHELL",
        "RSYNC_ICONV",
        "RSYNC_CHECKSUM_LIST",
        "RSYNC_COMPRESS_LIST",
        "RSYNC_MAX_ALLOC",
    ] {
        command.env_remove(name);
    }
    let parent = unsafe { libc::getpid() };
    unsafe {
        command.pre_exec(move || {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::getppid() != parent {
                libc::_exit(125);
            }
            Ok(())
        });
    }
    let mut child = command
        .spawn()
        .map_err(|_| "SSH/rsync executable unavailable")?;
    let pid = child.id() as i32;
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let reader = std::thread::spawn(move || {
        let (mut capture, mut line, mut block) = (Vec::new(), Vec::new(), [0; 4096]);
        while let Ok(n) = stdout.read(&mut block) {
            if n == 0 {
                break;
            }
            for &b in &block[..n] {
                if probe && capture.len() < 65536 {
                    capture.push(b);
                }
                if b == b'\r' || b == b'\n' {
                    progress_line(&line, &progress);
                    line.clear();
                } else if line.len() < 1024 {
                    line.push(b);
                }
            }
        }
        capture
    });
    let errors = std::thread::spawn(move || {
        let _ = std::io::copy(&mut stderr, &mut std::io::sink());
    });
    let deadline = Instant::now()
        + if probe {
            Duration::from_secs(30)
        } else {
            Duration::from_secs(3600)
        };
    let mut interrupted = false;
    let result = loop {
        if cancel.load(Ordering::Relaxed) || Instant::now() > deadline {
            interrupted = true;
            unsafe {
                libc::kill(-pid, libc::SIGTERM);
            }
            std::thread::sleep(Duration::from_millis(100));
            unsafe {
                libc::kill(-pid, libc::SIGKILL);
            }
            break child.wait().map_err(error);
        }
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => {
                unsafe {
                    libc::kill(-pid, libc::SIGKILL);
                }
                let _ = child.wait();
                break Err(error(e));
            }
        }
    };
    let captured = reader.join().map_err(|_| "Rsync output worker stopped")?;
    let _ = errors.join();
    if interrupted {
        return Err("Rsync cancelled or timed out; destination preserved".into());
    }
    if !result?.success() {
        return Err(if probe{"SSH/rsync probe failed; use SFTP for saved passwords or unlock the SSH key in your agent"}else{"Rsync transfer failed; destination preserved"}.into());
    }
    Ok(captured)
}
fn permissions(mode: u32) -> FileStat {
    FileStat {
        size: None,
        uid: None,
        gid: None,
        perm: Some(mode),
        atime: None,
        mtime: None,
    }
}
fn remote_names(sftp: &Sftp, path: &Path) -> Result<Vec<std::ffi::OsString>, String> {
    let mut directory = sftp
        .opendir(path)
        .map_err(|e| sftp_error("directory open", e))?;
    let mut names = Vec::new();
    loop {
        match directory.readdir() {
            Ok((p, _)) => {
                if p == Path::new(".") || p == Path::new("..") {
                    continue;
                }
                if names.len() >= MAX_ENTRIES {
                    return Err("Directory exceeds 10,000 entries".into());
                }
                names.push(p.file_name().ok_or("Invalid directory entry")?.to_owned());
            }
            Err(e)
                if e.code() == ssh2::ErrorCode::SFTP(1)
                    || e.code() == ssh2::ErrorCode::Session(-16) =>
            {
                break;
            }
            Err(e) => return Err(error(e)),
        }
    }
    Ok(names)
}
fn clean_remote(sftp: &Sftp, path: &Path, depth: usize) -> Result<(), String> {
    if depth > 80 {
        return Err("Owned staging cleanup exceeded depth limit".into());
    }
    let stat = sftp
        .lstat(path)
        .map_err(|e| sftp_error("cleanup metadata", e))?;
    if stat.perm.is_some_and(|m| m & 0o170000 == 0o040000) {
        sftp.setstat(path, permissions(0o700))
            .map_err(|e| sftp_error("cleanup permissions", e))?;
        for name in remote_names(sftp, path)? {
            let name = name.as_os_str();
            if name == "." || name == ".." {
                continue;
            }
            clean_remote(sftp, &path.join(name), depth + 1)?;
        }
        sftp.rmdir(path)
            .map_err(|e| sftp_error("staging directory removal", e))
    } else {
        sftp.unlink(path)
            .map_err(|e| sftp_error("staging file removal", e))
    }
}
fn clean_local(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(error)?;
    if metadata.is_dir() {
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(error)?;
        for entry in fs::read_dir(path).map_err(error)? {
            clean_local(&entry.map_err(error)?.path())?;
        }
        fs::remove_dir(path).map_err(error)
    } else {
        fs::remove_file(path).map_err(error)
    }
}
fn rename_local(from: &Path, to: &Path) -> Result<(), String> {
    let from = std::ffi::CString::new(from.as_os_str().as_bytes()).map_err(error)?;
    let to = std::ffi::CString::new(to.as_os_str().as_bytes()).map_err(error)?;
    if unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            libc::AT_FDCWD,
            from.as_ptr(),
            libc::AT_FDCWD,
            to.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    } != 0
    {
        return Err(error(std::io::Error::last_os_error()));
    }
    Ok(())
}
pub fn transfer(
    p: &Profile,
    local: &Path,
    remote: &Path,
    upload: bool,
    overwrite: bool,
    cancel: &AtomicBool,
    progress: Arc<AtomicU64>,
) -> Result<Receipt, String> {
    check(cancel)?;
    valid(local)?;
    valid(remote)?;
    let (args, host) = ssh_args(p)?;
    let mut local_probe = Command::new("rsync");
    local_probe.arg("--version");
    version(&run(local_probe, cancel, progress.clone(), true)?)?;
    let mut remote_probe = Command::new("ssh");
    remote_probe
        .args(&args)
        .args(["--", &host, "rsync", "--version"]);
    version(&run(remote_probe, cancel, progress.clone(), true)?)?;
    let session = crate::files::connect(p)?;
    let sftp = session.sftp().map_err(error)?;
    let remote = crate::files::remote_path(&sftp, remote, upload)?;
    let local = if upload {
        if fs::symlink_metadata(local)
            .map_err(error)?
            .file_type()
            .is_symlink()
        {
            return Err("Local source symlinks are refused".into());
        }
        fs::canonicalize(local).map_err(error)?
    } else {
        fs::canonicalize(
            local
                .parent()
                .ok_or("Choose a local destination directory")?,
        )
        .map_err(error)?
        .join(local.file_name().ok_or("Choose a destination name")?)
    };
    let (source, destination) = if upload {
        (&local, &remote)
    } else {
        (&remote, &local)
    };
    let manifest = scan(source, if upload { None } else { Some(&sftp) }, cancel)
        .map_err(|e| format!("Preparing source: {e}"))?;
    let root = manifest.get(Path::new("")).unwrap();
    let exists = if upload {
        match sftp.lstat(destination) {
            Ok(s) => {
                if s.perm.map(|m| m & 0o170000)
                    != Some(if root.directory { 0o040000 } else { 0o100000 })
                {
                    return Err("Destination type differs or is a symlink".into());
                }
                true
            }
            Err(e) if e.code() == ssh2::ErrorCode::SFTP(2) => false,
            Err(e) => return Err(error(e)),
        }
    } else {
        match fs::symlink_metadata(destination) {
            Ok(s) => {
                if s.file_type().is_symlink()
                    || s.is_dir() != root.directory
                    || (!s.is_dir() && !s.is_file())
                {
                    return Err("Destination type differs or is a symlink".into());
                }
                true
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => return Err(error(e)),
        }
    };
    if exists && !overwrite {
        return Err(
            "Destination exists; explicit whole-destination replacement is required".into(),
        );
    }
    let parent = destination.parent().ok_or("Choose a destination parent")?;
    let stage = parent.join(ruwac_protocol::id("ruwac-rsync-part"));
    if upload {
        sftp.mkdir(&stage, 0o700).map_err(error)?;
    } else {
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&stage)
            .map_err(error)?;
    }
    let filelist = std::env::temp_dir().join(ruwac_protocol::id("ruwac-rsync-list"));
    let result = (|| {
        let mut list = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&filelist)
            .map_err(error)?;
        let prefix = source
            .strip_prefix("/")
            .map_err(|_| "Rsync source must be absolute")?;
        if prefix.as_os_str().is_empty() {
            return Err("Select one source file or directory, not a filesystem root".into());
        }
        for relative in manifest.keys() {
            list.write_all(node(prefix, relative).as_os_str().as_bytes())
                .map_err(error)?;
            list.write_all(&[0]).map_err(error)?;
        }
        list.sync_all().map_err(error)?;
        drop(list);
        let remote_host = if host.contains(':') {
            format!("[{host}]")
        } else {
            host.clone()
        };
        let remote_arg = if upload {
            format!(
                "{remote_host}:{}/",
                stage.to_str().ok_or("Remote path must be UTF-8")?
            )
        } else {
            format!("{remote_host}:/")
        };
        let rsh = std::iter::once("ssh")
            .chain(args.iter().map(String::as_str))
            .map(rsh_quote)
            .collect::<Vec<_>>()
            .join(" ");
        let mut command = Command::new("rsync");
        command
            .args([
                "--secluded-args",
                "--from0",
                "--dirs",
                "--relative",
                "--no-implied-dirs",
                "--no-links",
                "--no-specials",
                "--no-devices",
                "--perms",
                "--times",
                "--chmod=D700,F600",
                "--info=progress2,name0",
                "--no-inc-recursive",
                "--max-alloc=67108864",
                "--max-size=34359738368",
                "--rsh",
            ])
            .arg(rsh)
            .arg("--files-from")
            .arg(&filelist)
            .arg("--");
        if upload {
            command.arg("/").arg(remote_arg);
        } else {
            command.arg(remote_arg).arg(format!("{}/", stage.display()));
        }
        run(command, cancel, progress.clone(), false)?;
        check(cancel)?;
        let payload = stage.join(prefix);
        let received = scan(&payload, if upload { Some(&sftp) } else { None }, cancel)?;
        if received.len() != manifest.len()
            || manifest.iter().any(|(path, e)| {
                received.get(path).is_none_or(|r| {
                    r.directory != e.directory || r.bytes != e.bytes || r.hash != e.hash
                })
            })
        {
            return Err("Rsync SHA-256/tree verification failed; destination preserved".into());
        }
        for (path, e) in manifest.iter().rev() {
            let target = node(&payload, path);
            if upload {
                sftp.setstat(&target, permissions(e.mode))
                    .map_err(|e| sftp_error("destination permissions", e))?;
            } else {
                fs::set_permissions(&target, fs::Permissions::from_mode(e.mode)).map_err(error)?;
            }
        }
        check(cancel)?;
        let backup = parent.join(ruwac_protocol::id("ruwac-rsync-backup"));
        if exists {
            if upload {
                sftp.rename(destination, &backup, Some(ssh2::RenameFlags::empty()))
                    .map_err(error)?;
            } else {
                rename_local(destination, &backup)?;
            }
        }
        let commit = if upload {
            sftp.rename(&payload, destination, Some(ssh2::RenameFlags::empty()))
                .map_err(error)
        } else {
            rename_local(&payload, destination)
        };
        if let Err(e) = commit {
            if exists {
                let restored = if upload {
                    sftp.rename(&backup, destination, Some(ssh2::RenameFlags::empty()))
                        .map_err(error)
                } else {
                    rename_local(&backup, destination)
                };
                if restored.is_err() {
                    return Err(format!(
                        "Destination changed during commit; original retained at {}",
                        backup.display()
                    ));
                }
            }
            return Err(e);
        }
        if exists {
            let cleanup = if upload {
                clean_remote(&sftp, &backup, 0)
            } else {
                if root.directory {
                    clean_local(&backup)
                } else {
                    fs::remove_file(&backup).map_err(error)
                }
            };
            if cleanup.is_err() {
                return Err(format!(
                    "Transfer committed and verified; original backup retained at {}",
                    backup.display()
                ));
            }
        }
        let (mut bytes, mut files, mut directories, mut hash) =
            (0u64, 0usize, 0usize, Sha256::new());
        for (path, e) in &manifest {
            hash.update((path.as_os_str().len() as u64).to_le_bytes());
            hash.update(path.as_os_str().as_bytes());
            hash.update([u8::from(e.directory)]);
            hash.update(e.mode.to_le_bytes());
            hash.update(e.bytes.to_le_bytes());
            hash.update(e.hash.as_bytes());
            bytes += e.bytes;
            if e.directory {
                directories += 1
            } else {
                files += 1
            };
        }
        progress.store(bytes, Ordering::Relaxed);
        Ok(Receipt {
            bytes,
            files,
            directories,
            sha256: format!("{:x}", hash.finalize()),
        })
    })();
    let _ = fs::remove_file(filelist);
    let cleaned = if upload {
        clean_remote(&sftp, &stage, 0)
    } else {
        clean_local(&stage)
    };
    if let Err(e) = cleaned {
        return Err(format!(
            "Owned staging cleanup failed at {} ({e}); prior result: {}; inspect before retrying",
            stage.display(),
            result
                .as_ref()
                .err()
                .map(String::as_str)
                .unwrap_or("committed and verified")
        ));
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_progress_and_rsync_quote_parser() {
        let n = AtomicU64::new(0);
        progress_line(b" 1,234 23% 5MB/s", &n);
        assert_eq!(n.load(Ordering::Relaxed), 1234);
        progress_line(b"999 filename", &n);
        assert_eq!(n.load(Ordering::Relaxed), 1234);
        assert_eq!(rsh_quote("a'b c"), "'a'\"'\"'b c'");
        assert!(version(b"rsync  version 3.5.1-g123 protocol version 33").is_ok());
        assert!(version(b"rsync  version 2.6.9").is_err());
        assert!(version(b"rsync  version 3.2.7").is_err());
        assert!(valid(Path::new("a/../b")).is_err());
    }
    #[test]
    fn local_no_replace_keeps_existing_destination() {
        let dir = std::env::temp_dir().join(ruwac_protocol::id("rsync-rename-fixture"));
        fs::create_dir(&dir).unwrap();
        let a = dir.join("a");
        let b = dir.join("b");
        fs::write(&a, b"new").unwrap();
        fs::write(&b, b"original").unwrap();
        assert!(rename_local(&a, &b).is_err());
        assert_eq!(fs::read(&b).unwrap(), b"original");
        fs::remove_file(&b).unwrap();
        rename_local(&a, &b).unwrap();
        assert_eq!(fs::read(&b).unwrap(), b"new");
        fs::remove_dir_all(dir).unwrap();
    }
}
