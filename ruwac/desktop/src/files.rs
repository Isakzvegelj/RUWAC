//! In-window SFTP, using libssh2. No shell interpolation or daemon payload transport.
use crate::profiles::{Profile, endpoint};
use gtk::prelude::*;
use sha2::{Digest, Sha256};
use ssh2::{CheckResult, KnownHostFileKind, OpenFlags, OpenType, RenameFlags, Session};
use std::{
    cell::RefCell,
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    path::{Component, Path, PathBuf},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};
const MAX_FILE: u64 = 32 * 1024 * 1024 * 1024;
fn error(_: impl std::fmt::Display) -> String {
    "SFTP operation failed (connection, permissions or server error)".into()
}
fn connect(p: &Profile) -> Result<Session, String> {
    let (host, port) = endpoint(&p.server, 22)?;
    let addr = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(error)?
        .next()
        .ok_or("SSH address unavailable")?;
    let tcp = TcpStream::connect_timeout(&addr, Duration::from_secs(5)).map_err(error)?;
    tcp.set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(error)?;
    tcp.set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(error)?;
    let mut session = Session::new().map_err(error)?;
    session.set_tcp_stream(tcp);
    session.set_timeout(5000);
    session.handshake().map_err(error)?;
    let (key, _) = session.host_key().ok_or("SSH host key unavailable")?;
    let path = if p.preferences.known_hosts.is_empty() {
        PathBuf::from(std::env::var_os("HOME").ok_or("Home unavailable")?).join(".ssh/known_hosts")
    } else {
        PathBuf::from(&p.preferences.known_hosts)
    };
    let mut known = session.known_hosts().map_err(error)?;
    known
        .read_file(&path, KnownHostFileKind::OpenSSH)
        .map_err(|_| "Known-hosts unavailable; verify the host explicitly in its terminal first")?;
    match known.check_port(&host, port, key) {
        CheckResult::Match => (),
        CheckResult::Mismatch => {
            return Err(
                "SSH host key CHANGED; verify with the host owner before updating trust".into(),
            );
        }
        _ => {
            return Err(
                "Unknown SSH host key; verify its fingerprint explicitly in the SSH terminal first"
                    .into(),
            );
        }
    }
    if !p.preferences.identity_file.is_empty() {
        let mut key = String::from_utf8(crate::credentials::bounded_file(
            Path::new(&p.preferences.identity_file),
            1024 * 1024,
        )?)
        .map_err(|_| "SSH key is not supported text")?;
        let result = session.userauth_pubkey_memory(&p.username, None, &key, None);
        unsafe { key.as_bytes_mut().fill(0) };
        result.map_err(error)?;
    } else {
        let agent = session.userauth_agent(&p.username);
        if agent.is_err() {
            let mut password = crate::credentials::lookup(&p.secret_reference)?;
            let result = session.userauth_password(&p.username, &password);
            unsafe { password.as_bytes_mut().fill(0) };
            result.map_err(|_| "SSH authentication rejected")?;
        }
    }
    if !session.authenticated() {
        return Err("SSH authentication rejected".into());
    }
    Ok(session)
}
fn remote_path(sftp: &ssh2::Sftp, path: &Path, allow_missing: bool) -> Result<PathBuf, String> {
    if path.as_os_str().len() > 4096
        || path.components().any(|c| matches!(c, Component::ParentDir))
        || path.to_string_lossy().contains('\0')
    {
        return Err("Invalid remote path".into());
    }
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        sftp.realpath(Path::new(".")).map_err(error)?.join(path)
    };
    let mut current = PathBuf::from("/");
    let parts: Vec<_> = absolute.components().collect();
    for (i, c) in parts.iter().enumerate() {
        if matches!(c, Component::RootDir | Component::CurDir) {
            continue;
        }
        current.push(c.as_os_str());
        match sftp.lstat(&current) {
            Ok(stat) => {
                if stat.perm.is_some_and(|p| p & 0o170000 == 0o120000) {
                    return Err(
                        "Remote symlink paths require a verified real directory; transfer refused"
                            .into(),
                    );
                }
            }
            Err(e)
                if allow_missing
                    && i == parts.len() - 1
                    && e.code() == ssh2::ErrorCode::SFTP(2) =>
            {
                ()
            }
            Err(e) => return Err(error(e)),
        }
    }
    Ok(absolute)
}
#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub directory: bool,
    pub symlink: bool,
    pub permissions: u32,
    pub bytes: u64,
}
pub fn list(p: &Profile, path: &Path) -> Result<Vec<Entry>, String> {
    list_inner(p, path, &AtomicBool::new(false))
}
fn list_inner(p: &Profile, path: &Path, cancel: &AtomicBool) -> Result<Vec<Entry>, String> {
    let session = connect(p)?;
    let sftp = session.sftp().map_err(error)?;
    let path = remote_path(&sftp, path, false)?;
    let mut dir = sftp.opendir(&path).map_err(error)?;
    let mut items = Vec::new();
    let start = std::time::Instant::now();
    loop {
        if cancel.load(Ordering::Relaxed) || start.elapsed() > Duration::from_secs(15) {
            return Err("Directory listing cancelled or timed out".into());
        }
        match dir.readdir() {
            Ok((p, s)) => {
                let name = p.to_string_lossy().to_string();
                if name == "." || name == ".." {
                    continue;
                }
                if items.len() >= 1000 {
                    return Err("Directory exceeds 1,000-entry display limit".into());
                }
                let mode = s.perm.unwrap_or(0);
                items.push(Entry {
                    name,
                    directory: mode & 0o170000 == 0o040000,
                    symlink: mode & 0o170000 == 0o120000,
                    permissions: mode & 0o7777,
                    bytes: s.size.unwrap_or(0),
                });
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
    items.sort_by_key(|e| (!e.directory, e.name.clone()));
    Ok(items)
}
fn copy(
    reader: &mut impl Read,
    writer: &mut impl Write,
    cancel: &AtomicBool,
    progress: &AtomicU64,
) -> Result<String, String> {
    let mut bytes = [0u8; 65536];
    let mut hash = Sha256::new();
    let mut count = 0;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("Transfer cancelled; partial file removed".into());
        }
        let n = reader.read(&mut bytes).map_err(error)?;
        if n == 0 {
            break;
        }
        count += n as u64;
        if count > MAX_FILE {
            return Err("Transfer exceeds 32 GiB cap".into());
        }
        writer.write_all(&bytes[..n]).map_err(error)?;
        hash.update(&bytes[..n]);
        progress.store(count, Ordering::Relaxed);
    }
    writer.flush().map_err(error)?;
    Ok(format!("{:x}", hash.finalize()))
}
fn verify_remote(
    sftp: &ssh2::Sftp,
    path: &Path,
    hash: &str,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let mut file = sftp.open(path).map_err(error)?;
    let mut sink = std::io::sink();
    let digest = copy(&mut file, &mut sink, cancel, &AtomicU64::new(0))?;
    if digest != hash {
        return Err("SFTP SHA-256 verification mismatch".into());
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
    progress: &AtomicU64,
) -> Result<String, String> {
    use std::os::unix::fs::OpenOptionsExt;
    if cancel.load(Ordering::Relaxed) {
        return Err("Transfer cancelled".into());
    }
    let session = connect(p)?;
    let sftp = session.sftp().map_err(error)?;
    let remote = remote_path(&sftp, remote, upload)?;
    if upload {
        match sftp.lstat(&remote) {
            Ok(s) => {
                if !overwrite {
                    return Err("Destination exists; choose Overwrite, Rename or Skip".into());
                }
                if s.perm.is_none_or(|p| p & 0o170000 != 0o100000) {
                    return Err("Destination is not a regular file".into());
                }
            }
            Err(e) if e.code() == ssh2::ErrorCode::SFTP(2) => (),
            Err(e) => return Err(error(e)),
        }
        let mut input = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(local)
            .map_err(error)?;
        if !input
            .metadata()
            .is_ok_and(|m| m.is_file() && m.len() <= MAX_FILE)
        {
            return Err("Local source must be a bounded regular file".into());
        }
        let tmp = remote.with_file_name(ruwac_protocol::id("ruwac-part"));
        let result = (|| {
            let mut out = sftp
                .open_mode(
                    &tmp,
                    OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::EXCLUSIVE,
                    0o600,
                    OpenType::File,
                )
                .map_err(error)?;
            let hash = copy(&mut input, &mut out, cancel, progress)?;
            out.close().map_err(error)?;
            verify_remote(&sftp, &tmp, &hash, cancel)?;
            if cancel.load(Ordering::Relaxed) {
                return Err("Transfer cancelled".into());
            }
            if overwrite && sftp.lstat(&remote).is_ok() {
                // SFTP v3 has no portable atomic overwrite. Preserve the old regular file
                // until the verified replacement commits; never unlink it preemptively.
                let previous = remote.with_file_name(ruwac_protocol::id("ruwac-backup"));
                sftp.rename(&remote, &previous, Some(RenameFlags::empty()))
                    .map_err(error)?;
                if sftp
                    .rename(&tmp, &remote, Some(RenameFlags::empty()))
                    .is_err()
                {
                    if sftp
                        .rename(&previous, &remote, Some(RenameFlags::empty()))
                        .is_err()
                    {
                        return Err("Replacement interrupted; original preserved as ruwac-backup in the remote directory".into());
                    }
                    return Err("Replacement failed; original remote file restored".into());
                }
                sftp.unlink(&previous).map_err(
                    |_| "Replacement verified; old ruwac-backup retained because cleanup failed",
                )?;
            } else {
                sftp.rename(&tmp, &remote, Some(RenameFlags::empty()))
                    .map_err(error)?;
            }
            Ok(hash)
        })();
        if result.is_err() {
            let _ = sftp.unlink(&tmp);
        }
        result
    } else {
        if !sftp
            .lstat(&remote)
            .map_err(error)?
            .perm
            .is_some_and(|p| p & 0o170000 == 0o100000)
        {
            return Err("Remote source must be a regular file".into());
        }
        if let Ok(m) = std::fs::symlink_metadata(local) {
            if !overwrite {
                return Err("Destination exists; choose Overwrite, Rename or Skip".into());
            }
            if !m.is_file() {
                return Err("Local destination is not a regular file".into());
            }
        }
        let parent = local.parent().ok_or("Select a local directory")?;
        let canonical = std::fs::canonicalize(parent).map_err(error)?;
        let destination = canonical.join(local.file_name().ok_or("Select a file name")?);
        let tmp = canonical.join(ruwac_protocol::id("ruwac-part"));
        let result = (|| {
            let mut out = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW)
                .open(&tmp)
                .map_err(error)?;
            let mut input = sftp.open(&remote).map_err(error)?;
            let hash = copy(&mut input, &mut out, cancel, progress)?;
            out.sync_all().map_err(error)?;
            drop(out);
            if cancel.load(Ordering::Relaxed) {
                return Err("Transfer cancelled".into());
            }
            if overwrite {
                std::fs::rename(&tmp, &destination).map_err(error)?;
            } else {
                std::fs::hard_link(&tmp, &destination).map_err(error)?;
                std::fs::remove_file(&tmp).map_err(error)?;
            }
            Ok(hash)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(tmp);
        }
        result
    }
}
pub fn panel(profile: Profile) -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 6);
    root.set_tooltip_text(Some(&format!("{} · Files", profile.name)));
    let title = gtk::Label::new(Some(&format!(
        "Remote SFTP · {} / {} · verified SSH host keys",
        profile.group, profile.name
    )));
    root.append(&title);
    let remote = gtk::Entry::builder()
        .text(".")
        .placeholder_text("Remote directory")
        .build();
    root.append(&remote);
    let controls = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    root.append(&controls);
    let list = gtk::Box::new(gtk::Orientation::Vertical, 4);
    root.append(
        &gtk::ScrolledWindow::builder()
            .vexpand(true)
            .child(&list)
            .build(),
    );
    let destination = gtk::Entry::builder()
        .placeholder_text("Remote file name (select a row or enter a name)")
        .build();
    root.append(&destination);
    let overwrite =
        gtk::CheckButton::with_label("Overwrite existing regular file (explicit consent)");
    root.append(&overwrite);
    let status = gtk::Label::new(Some(
        "Local files are chosen with a native dialog. Transfers are verified with SHA-256.",
    ));
    status.set_wrap(true);
    root.append(&status);
    let progress = gtk::ProgressBar::new();
    progress.set_show_text(true);
    root.append(&progress);
    let busy = Rc::new(RefCell::new(None::<Arc<AtomicBool>>));
    let refresh = gtk::Button::with_label("Refresh");
    controls.append(&refresh);
    let up = gtk::Button::with_label("Up");
    controls.append(&up);
    let e = remote.clone();
    let r = refresh.clone();
    up.connect_clicked(move |_| {
        let path = PathBuf::from(e.text().as_str());
        e.set_text(
            &path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("/"))
                .to_string_lossy(),
        );
        r.emit_clicked();
    });
    let (p, l, st, e, d, r, b) = (
        profile.clone(),
        list.clone(),
        status.clone(),
        remote.clone(),
        destination.clone(),
        refresh.clone(),
        busy.clone(),
    );
    refresh.connect_clicked(move |_| {
        if b.borrow().is_some() {
            st.set_text("A file operation is already running");
            return;
        }
        let path = PathBuf::from(e.text().as_str());
        let profile = p.clone();
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let token = Arc::new(AtomicBool::new(false));
        *b.borrow_mut() = Some(token.clone());
        st.set_text("Listing remote directory…");
        std::thread::spawn(move || {
            let _ = tx.send(list_inner(&profile, &path, &token));
        });
        let (l, st, e, d, r, b) = (
            l.clone(),
            st.clone(),
            e.clone(),
            d.clone(),
            r.clone(),
            b.clone(),
        );
        gtk::glib::timeout_add_local(Duration::from_millis(30), move || match rx.try_recv() {
            Ok(result) => {
                b.borrow_mut().take();
                match result {
                    Ok(rows) => {
                        while let Some(w) = l.first_child() {
                            l.remove(&w);
                        }
                        for item in rows {
                            let button = gtk::Button::with_label(&format!(
                                "{}  {:04o}  {} bytes{}",
                                item.name,
                                item.permissions,
                                item.bytes,
                                if item.directory {
                                    " /"
                                } else if item.symlink {
                                    " (symlink; blocked)"
                                } else {
                                    ""
                                }
                            ));
                            let (e, d, r) = (e.clone(), d.clone(), r.clone());
                            button.set_sensitive(!item.symlink);
                            button.connect_clicked(move |_| {
                                if item.directory {
                                    let path = PathBuf::from(e.text().as_str()).join(&item.name);
                                    e.set_text(&path.to_string_lossy());
                                    r.emit_clicked();
                                } else {
                                    d.set_text(&item.name);
                                }
                            });
                            l.append(&button);
                        }
                        st.set_text("Remote directory ready");
                    }
                    Err(err) => st.set_text(&err),
                }
                gtk::glib::ControlFlow::Break
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => gtk::glib::ControlFlow::Continue,
            Err(_) => {
                b.borrow_mut().take();
                st.set_text("SFTP worker stopped");
                gtk::glib::ControlFlow::Break
            }
        });
    });
    for (label, upload) in [("Upload from local…", true), ("Download to local…", false)] {
        let button = gtk::Button::with_label(label);
        controls.append(&button);
        let (p, e, d, st, b, prog, ow) = (
            profile.clone(),
            remote.clone(),
            destination.clone(),
            status.clone(),
            busy.clone(),
            progress.clone(),
            overwrite.clone(),
        );
        button.connect_clicked(move |button| {
            if b.borrow().is_some() {
                st.set_text("A file operation is already running");
                return;
            }
            let name = d.text();
            if name.is_empty() || name.contains('/') || name == ".." || name == "." {
                st.set_text("Enter one remote file name; navigate directories above");
                return;
            }
            let remote = PathBuf::from(e.text().as_str()).join(name.as_str());
            let parent = button.root().and_then(|w| w.downcast::<gtk::Window>().ok());
            let dialog = gtk::FileDialog::new();
            dialog.set_title(if upload {
                "Choose local upload source"
            } else {
                "Choose local download destination"
            });
            if !upload {
                dialog.set_initial_name(Some(name.as_str()));
            }
            let (p, st, b, prog, overwrite) = (
                p.clone(),
                st.clone(),
                b.clone(),
                prog.clone(),
                ow.is_active(),
            );
            let done = move |result: Result<gtk::gio::File, gtk::glib::Error>| {
                let Ok(file) = result else {
                    return;
                };
                let Some(local) = file.path() else {
                    return;
                };
                if b.borrow().is_some() {
                    st.set_text("A file operation is already running");
                    return;
                }
                let cancel = Arc::new(AtomicBool::new(false));
                let bytes = Arc::new(AtomicU64::new(0));
                *b.borrow_mut() = Some(cancel.clone());
                let (tx, rx) = std::sync::mpsc::sync_channel(1);
                let count = bytes.clone();
                std::thread::spawn(move || {
                    let _ = tx.send(transfer(
                        &p, &local, &remote, upload, overwrite, &cancel, &count,
                    ));
                });
                st.set_text("Transferring and verifying…");
                gtk::glib::timeout_add_local(Duration::from_millis(50), move || {
                    prog.pulse();
                    prog.set_text(Some(&format!(
                        "{} bytes transferred",
                        bytes.load(Ordering::Relaxed)
                    )));
                    match rx.try_recv() {
                        Ok(r) => {
                            b.borrow_mut().take();
                            prog.set_fraction(if r.is_ok() { 1.0 } else { 0.0 });
                            st.set_text(&match r {
                                Ok(hash) => format!("Transfer verified · SHA-256 {hash}"),
                                Err(e) => e,
                            });
                            gtk::glib::ControlFlow::Break
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => {
                            gtk::glib::ControlFlow::Continue
                        }
                        Err(_) => {
                            b.borrow_mut().take();
                            st.set_text("Transfer worker stopped");
                            gtk::glib::ControlFlow::Break
                        }
                    }
                });
            };
            if upload {
                dialog.open(parent.as_ref(), None::<&gtk::gio::Cancellable>, done);
            } else {
                dialog.save(parent.as_ref(), None::<&gtk::gio::Cancellable>, done);
            }
        });
    }
    let cancel = gtk::Button::with_label("Cancel / skip");
    controls.append(&cancel);
    let b = busy.clone();
    cancel.connect_clicked(move |_| {
        if let Some(token) = b.borrow().as_ref() {
            token.store(true, Ordering::Relaxed);
        }
    });
    let b = busy.clone();
    root.connect_unrealize(move |w| {
        let (weak, b) = (w.downgrade(), b.clone());
        gtk::glib::idle_add_local_once(move || {
            if weak.upgrade().is_none_or(|w| w.root().is_none()) {
                if let Some(token) = b.borrow().as_ref() {
                    token.store(true, Ordering::Relaxed);
                }
            }
        });
    });
    root
}

pub fn fixture(root: &gtk::Box, path: Option<&str>, refresh: bool) -> serde_json::Value {
    fn walk(
        w: &gtk::Widget,
        entries: &mut Vec<gtk::Entry>,
        buttons: &mut Vec<gtk::Button>,
        labels: &mut Vec<String>,
    ) {
        if let Some(e) = w.downcast_ref::<gtk::Entry>() {
            entries.push(e.clone());
        }
        if let Some(b) = w.downcast_ref::<gtk::Button>() {
            buttons.push(b.clone());
        }
        if let Some(l) = w.downcast_ref::<gtk::Label>() {
            labels.push(l.text().to_string());
        }
        let mut c = w.first_child();
        while let Some(w) = c {
            c = w.next_sibling();
            walk(&w, entries, buttons, labels);
        }
    }
    let (mut entries, mut buttons, mut labels) = (vec![], vec![], vec![]);
    walk(root.upcast_ref(), &mut entries, &mut buttons, &mut labels);
    if let (Some(path), Some(e)) = (path, entries.first()) {
        e.set_text(path);
    }
    if refresh {
        if let Some(b) = buttons
            .iter()
            .find(|b| b.label().as_deref() == Some("Refresh"))
        {
            b.emit_clicked();
        }
    }
    serde_json::json!({"labels":labels,"buttons":buttons.iter().map(|b|b.label().unwrap_or_default().to_string()).collect::<Vec<_>>()})
}

/// The current directory is nonsecret workspace metadata; restoring it does not connect.
pub fn directory(root: &gtk::Box, value: Option<&str>) -> String {
    let mut child = root.first_child();
    while let Some(w) = child {
        child = w.next_sibling();
        if let Some(e) = w.downcast_ref::<gtk::Entry>() {
            if let Some(value) = value {
                e.set_text(value);
            }
            return e.text().to_string();
        }
    }
    ".".into()
}
