//! Explicit, durable SFTP resume jobs. Partial files never replace destinations until verified.
use crate::{files, profiles::Profile};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ssh2::{OpenFlags, OpenType, RenameFlags};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    os::{
        fd::AsRawFd,
        unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};
const MAX_FILE: u64 = 32 * 1024 * 1024 * 1024;
const MAX_JOBS: usize = 128;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Stamp {
    size: u64,
    mtime: u64,
    mode: u32,
    uid: u32,
    identity: Option<(u64, u64)>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pending {
    pub id: String,
    pub upload: bool,
    pub local: PathBuf,
    pub remote: PathBuf,
    endpoint: String,
    binding: String,
    host_key_sha256: String,
    overwrite: bool,
    source: Stamp,
    sha256: String,
    destination: Option<(Stamp, String)>,
    pub phase: String,
    schema: u8,
    part_uid: Option<u32>,
}
#[derive(Serialize)]
pub struct Receipt {
    pub id: String,
    pub sha256: String,
    pub bytes: u64,
    pub resumed_from: u64,
}
fn stopped(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) {
        Err("Transfer cancelled".into())
    } else {
        Ok(())
    }
}
pub(super) fn binding(p: &Profile) -> String {
    format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&serde_json::json!([
                p.id,
                p.server,
                p.username,
                p.preferences.identity_file,
                p.preferences.identity_secret_reference,
                p.preferences.known_hosts
            ]))
            .unwrap()
        )
    )
}
pub(super) fn host_key(session: &ssh2::Session) -> Result<String, String> {
    let (key, _) = session.host_key().ok_or("SSH host identity unavailable")?;
    Ok(format!("{:x}", Sha256::digest(key)))
}
fn original_host(session: &ssh2::Session, job: &Pending) -> Result<(), String> {
    if host_key(session)? != job.host_key_sha256 {
        Err("Original SSH host key changed; pending transfer cannot be retargeted".into())
    } else {
        Ok(())
    }
}
pub(super) fn private_dir(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| {
        m.is_dir()
            && !m.file_type().is_symlink()
            && m.uid() == unsafe { libc::geteuid() }
            && m.mode() & 0o777 == 0o700
    })
}
pub(super) fn root() -> Result<PathBuf, String> {
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/state")))
        .ok_or("State directory unavailable")?;
    fs::create_dir_all(&state).map_err(files::error)?;
    let app = state.join("RUWAC");
    let path = app.join("transfers");
    for dir in [&app, &path] {
        match fs::DirBuilder::new().mode(0o700).create(dir) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(files::error(e)),
        };
        if !private_dir(dir) {
            return Err("Transfer journal needs an owned, nonsymlink 0700 directory".into());
        }
    }
    Ok(path)
}
fn regular(path: &Path, write: bool) -> Result<File, String> {
    let f = OpenOptions::new()
        .read(true)
        .write(write)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(files::error)?;
    if !f
        .metadata()
        .is_ok_and(|m| m.is_file() && m.len() <= MAX_FILE)
    {
        return Err("Select a bounded regular file without a symlink".into());
    }
    Ok(f)
}
pub(super) fn private_file(f: &File) -> bool {
    f.metadata().is_ok_and(|m| {
        m.is_file()
            && m.uid() == unsafe { libc::geteuid() }
            && m.mode() & 0o777 == 0o600
            && m.nlink() == 1
    })
}
fn namespace() -> Result<File, String> {
    let path = root()?.join(".namespace.lock");
    let f = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(files::error)?;
    if !private_file(&f)
        || unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0
    {
        return Err("Transfer journal is busy; retry shortly".into());
    }
    Ok(f)
}
struct Journal {
    dir: PathBuf,
    id: String,
    lock: File,
}
impl Journal {
    fn open(id: &str) -> Result<Self, String> {
        if !ruwac_protocol::valid_id(id, "transfer") {
            return Err("Invalid transfer ID".into());
        }
        let dir = root()?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(dir.join(format!("{id}.lock")))
            .map_err(files::error)?;
        if !private_file(&lock)
            || unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0
        {
            return Err("Transfer is busy or its lock ownership is invalid".into());
        }
        Ok(Self {
            dir,
            id: id.into(),
            lock,
        })
    }
    fn path(&self) -> PathBuf {
        self.dir.join(format!("{}.json", self.id))
    }
    fn load(&self, p: &Profile) -> Result<Pending, String> {
        let job = read_job(&self.path())?;
        if job.endpoint != p.id || !job.eligible(p) {
            return Err("Transfer profile settings changed; restore the original endpoint settings before Resume/Discard".into());
        }
        Ok(job)
    }
    fn save(&self, job: &Pending) -> Result<(), String> {
        if !private_dir(&self.dir) {
            return Err("Transfer journal ownership changed".into());
        }
        if self.path().exists() {
            let f = regular(&self.path(), false)?;
            if !private_file(&f) {
                return Err("Transfer journal ownership invalid".into());
            }
        }
        let tmp = self.dir.join(ruwac_protocol::id("transfer-write"));
        let data = serde_json::to_vec(job).map_err(files::error)?;
        if data.len() > 32768 {
            return Err("Transfer metadata exceeds its limit".into());
        }
        let result = (|| {
            let mut f = OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW)
                .open(&tmp)
                .map_err(files::error)?;
            f.write_all(&data).map_err(files::error)?;
            f.sync_all().map_err(files::error)?;
            fs::rename(&tmp, self.path()).map_err(files::error)?;
            File::open(&self.dir)
                .and_then(|f| f.sync_all())
                .map_err(files::error)
        })();
        if result.is_err() {
            let _ = fs::remove_file(tmp);
        }
        result
    }
    fn remove(&self) -> Result<(), String> {
        let f = regular(&self.path(), false)?;
        if !private_file(&f) {
            return Err("Transfer journal ownership invalid".into());
        }
        fs::remove_file(self.path()).map_err(files::error)?;
        File::open(&self.dir)
            .and_then(|f| f.sync_all())
            .map_err(files::error)
    }
}
impl Drop for Journal {
    fn drop(&mut self) {
        // Keep a paused job's lock inode stable across competing processes.
        // Removing it while its journal survives could split ownership between
        // an already-open descriptor and a newly-created inode.
        if self.path().exists() {
            return;
        }
        let path = self.dir.join(format!("{}.lock", self.id));
        if let (Ok(a), Ok(b)) = (self.lock.metadata(), fs::symlink_metadata(&path)) {
            if a.ino() == b.ino() && a.dev() == b.dev() {
                let _ = fs::remove_file(path);
            }
        }
    }
}
fn read_job(path: &Path) -> Result<Pending, String> {
    let f = regular(path, false)?;
    if !private_file(&f) || f.metadata().map_err(files::error)?.len() > 32768 {
        return Err("Transfer journal ownership or size invalid".into());
    }
    let job: Pending = serde_json::from_reader(f.take(32769))
        .map_err(|_| "Transfer journal is invalid; original preserved")?;
    if job.schema != 1
        || !ruwac_protocol::valid_id(&job.id, "transfer")
        || path.file_name().and_then(|n| n.to_str()) != Some(&format!("{}.json", job.id))
        || job.endpoint.is_empty()
        || job.endpoint.len() > 1024
        || job.binding.len() != 64
        || job.host_key_sha256.len() != 64
        || job.source.size > MAX_FILE
        || !matches!(job.phase.as_str(), "copy" | "commit")
        || job.sha256.len() != 64
        || !job.sha256.bytes().all(|b| b.is_ascii_hexdigit())
        || [&job.local, &job.remote].iter().any(|p| {
            !p.is_absolute()
                || p.as_os_str().len() > 4096
                || p.components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
        })
    {
        return Err(
            "Transfer target/profile changed or journal is invalid; original preserved".into(),
        );
    }
    Ok(job)
}
impl Pending {
    pub fn eligible(&self, p: &Profile) -> bool {
        self.binding == binding(p)
    }
}
pub fn list(p: &Profile) -> Result<Vec<Pending>, String> {
    let dir = root()?;
    let mut jobs = Vec::new();
    let mut entries = 0;
    for entry in fs::read_dir(dir).map_err(files::error)? {
        let path = entry.map_err(files::error)?.path();
        if path.extension().is_none_or(|s| s != "json") {
            continue;
        }
        entries += 1;
        if entries > MAX_JOBS {
            return Err("Transfer journal exceeds its 128-job limit".into());
        }
        let job = read_job(&path)?;
        if job.endpoint == p.id {
            jobs.push(job);
        }
    }
    jobs.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(jobs)
}
fn local_stamp(f: &File) -> Result<Stamp, String> {
    let m = f.metadata().map_err(files::error)?;
    Ok(Stamp {
        size: m.len(),
        mtime: (m.mtime().max(0) as u64)
            .saturating_mul(1_000_000_000)
            .saturating_add(m.mtime_nsec().max(0) as u64),
        mode: m.mode() & 0o7777,
        uid: m.uid(),
        identity: Some((m.dev(), m.ino())),
    })
}
fn remote_stamp(s: &ssh2::Sftp, path: &Path) -> Result<Stamp, String> {
    let m = s.lstat(path).map_err(files::error)?;
    if m.perm.is_none_or(|p| p & 0o170000 != 0o100000) || m.size.is_none_or(|n| n > MAX_FILE) {
        return Err("Remote path must be a bounded regular file".into());
    }
    Ok(Stamp {
        size: m.size.unwrap(),
        mtime: m.mtime.ok_or("Remote file timestamp unavailable")?,
        mode: m.perm.unwrap() & 0o7777,
        uid: m.uid.ok_or("Remote owner unavailable")?,
        identity: None,
    })
}
enum Stream {
    Local(File),
    Remote(ssh2::File),
}
impl Read for Stream {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Local(f) => f.read(b),
            Self::Remote(f) => f.read(b),
        }
    }
}
impl Write for Stream {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Local(f) => f.write(b),
            Self::Remote(f) => f.write(b),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Local(f) => f.flush(),
            Self::Remote(f) => f.flush(),
        }
    }
}
impl Seek for Stream {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        match self {
            Self::Local(f) => f.seek(pos),
            Self::Remote(f) => f.seek(pos),
        }
    }
}
impl Stream {
    fn finish(mut self) -> Result<(), String> {
        self.flush().map_err(files::error)?;
        match self {
            Self::Local(f) => f.sync_all().map_err(files::error),
            Self::Remote(mut f) => f.close().map_err(files::error),
        }
    }
}
fn hash(reader: &mut impl Read, limit: u64, cancel: &AtomicBool) -> Result<(Sha256, u64), String> {
    let mut h = Sha256::new();
    let mut b = [0u8; 65536];
    let mut count = 0;
    while count < limit {
        stopped(cancel)?;
        let n = reader
            .read(&mut b[..((limit - count).min(65536)) as usize])
            .map_err(files::error)?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
        count += n as u64;
    }
    Ok((h, count))
}
fn full_hash(reader: &mut impl Read, cancel: &AtomicBool) -> Result<String, String> {
    let (h, n) = hash(reader, MAX_FILE + 1, cancel)?;
    if n > MAX_FILE {
        return Err("Transfer exceeds 32 GiB cap".into());
    }
    Ok(format!("{:x}", h.finalize()))
}
fn local_path(path: &Path) -> Result<PathBuf, String> {
    if path.as_os_str().len() > 4096
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("Invalid local file path".into());
    }
    Ok(fs::canonicalize(
        path.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )
    .map_err(files::error)?
    .join(path.file_name().ok_or("Select a file name")?))
}
fn part(job: &Pending) -> PathBuf {
    let dest = if job.upload { &job.remote } else { &job.local };
    dest.with_file_name(format!("ruwac-part-{}", job.id))
}
fn backup(job: &Pending) -> PathBuf {
    job.remote
        .with_file_name(format!("ruwac-backup-{}", job.id))
}
fn source(s: &ssh2::Sftp, job: &Pending) -> Result<(Stream, Stamp), String> {
    if job.upload {
        let f = regular(&job.local, false)?;
        let stamp = local_stamp(&f)?;
        Ok((Stream::Local(f), stamp))
    } else {
        let stamp = remote_stamp(s, &job.remote)?;
        Ok((
            Stream::Remote(s.open(&job.remote).map_err(files::error)?),
            stamp,
        ))
    }
}
fn destination(
    s: &ssh2::Sftp,
    job: &Pending,
    cancel: &AtomicBool,
) -> Result<Option<(Stamp, String)>, String> {
    if job.upload {
        match s.lstat(&job.remote) {
            Err(e) if e.code() == ssh2::ErrorCode::SFTP(2) => return Ok(None),
            Err(e) => return Err(files::error(e)),
            Ok(_) => (),
        };
        let stamp = remote_stamp(s, &job.remote)?;
        let digest = full_hash(&mut s.open(&job.remote).map_err(files::error)?, cancel)?;
        Ok(Some((stamp, digest)))
    } else {
        match fs::symlink_metadata(&job.local) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(files::error(e)),
            Ok(_) => (),
        };
        let mut f = regular(&job.local, false)?;
        let stamp = local_stamp(&f)?;
        let digest = full_hash(&mut f, cancel)?;
        Ok(Some((stamp, digest)))
    }
}
pub fn start(
    p: &Profile,
    local: &Path,
    remote: &Path,
    upload: bool,
    overwrite: bool,
    cancel: &AtomicBool,
    progress: &AtomicU64,
) -> Result<Receipt, String> {
    stopped(cancel)?;
    let dir = root()?;
    if fs::read_dir(&dir)
        .map_err(files::error)?
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|s| s == "json"))
        .count()
        >= MAX_JOBS
    {
        return Err("128 pending transfers; resume or discard one first".into());
    }
    let session = files::connect(p)?;
    let sftp = session.sftp().map_err(files::error)?;
    let local = local_path(local)?;
    let remote = files::remote_path(&sftp, remote, upload)?;
    let mut job = Pending {
        id: ruwac_protocol::id("transfer"),
        endpoint: p.id.clone(),
        binding: binding(p),
        host_key_sha256: host_key(&session)?,
        upload,
        local,
        remote,
        overwrite,
        source: Stamp {
            size: 0,
            mtime: 0,
            mode: 0,
            uid: 0,
            identity: None,
        },
        sha256: String::new(),
        destination: None,
        phase: "copy".into(),
        schema: 1,
        part_uid: None,
    };
    let (mut input, stamp) = source(&sftp, &job)?;
    job.source = stamp;
    job.sha256 = full_hash(&mut input, cancel)?;
    let (_, after) = source(&sftp, &job)?;
    if job.source != after {
        return Err("Source changed during preparation; start again".into());
    }
    job.destination = destination(&sftp, &job, cancel)?;
    if job.destination.is_some() && !overwrite {
        return Err("Destination exists; explicit overwrite consent is required".into());
    }
    let quota = namespace()?;
    if fs::read_dir(&dir)
        .map_err(files::error)?
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|s| s == "json"))
        .count()
        >= MAX_JOBS
    {
        return Err("128 pending transfers; resume or discard one first".into());
    }
    let journal = Journal::open(&job.id)?;
    journal.save(&job)?;
    drop(quota);
    drop(input);
    drop(sftp);
    drop(session);
    execute(p, job, journal, false, cancel, progress)
}
pub fn resume(
    p: &Profile,
    id: &str,
    cancel: &AtomicBool,
    progress: &AtomicU64,
) -> Result<Receipt, String> {
    let journal = Journal::open(id)?;
    let job = journal.load(p)?;
    execute(p, job, journal, true, cancel, progress)
}
fn partial(s: &ssh2::Sftp, job: &mut Pending, journal: &Journal) -> Result<(Stream, u64), String> {
    let path = part(job);
    if job.upload {
        let existing = match s.lstat(&path) {
            Ok(_) => Some(remote_stamp(s, &path)?),
            Err(e) if e.code() == ssh2::ErrorCode::SFTP(2) => None,
            Err(e) => return Err(files::error(e)),
        };
        if let Some(stamp) = &existing {
            if stamp.mode != 0o600
                || stamp.size > job.source.size
                || job.part_uid.is_some_and(|uid| uid != stamp.uid)
            {
                return Err(
                    "Remote partial ownership, permissions or size changed; preserved".into(),
                );
            }
        }
        let flags = if existing.is_some() {
            OpenFlags::READ | OpenFlags::WRITE
        } else {
            OpenFlags::READ | OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::EXCLUSIVE
        };
        let file = s
            .open_mode(&path, flags, 0o600, OpenType::File)
            .map_err(files::error)?;
        let stamp = remote_stamp(s, &path)?;
        if stamp.mode != 0o600 {
            return Err("Remote partial did not retain private 0600 permissions; preserved".into());
        }
        job.part_uid = Some(stamp.uid);
        journal.save(job)?;
        Ok((Stream::Remote(file), stamp.size))
    } else {
        let file = match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&path)
        {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => regular(&path, true)?,
            Err(e) => return Err(files::error(e)),
        };
        if !private_file(&file) {
            return Err("Local partial ownership or permissions changed; preserved".into());
        }
        let size = file.metadata().map_err(files::error)?.len();
        if size > job.source.size {
            return Err("Local partial is larger than its source; preserved".into());
        }
        Ok((Stream::Local(file), size))
    }
}
fn backup_valid(s: &ssh2::Sftp, job: &Pending, cancel: &AtomicBool) -> Result<bool, String> {
    match s.lstat(&backup(job)) {
        Err(e) if e.code() == ssh2::ErrorCode::SFTP(2) => return Ok(false),
        Err(e) => return Err(files::error(e)),
        Ok(_) => (),
    }
    let old = job
        .destination
        .as_ref()
        .ok_or("Unexpected backup file preserved")?;
    let stamp = remote_stamp(s, &backup(job))?;
    let digest = full_hash(&mut s.open(&backup(job)).map_err(files::error)?, cancel)?;
    if stamp != old.0 || digest != old.1 {
        return Err("Backup changed; preserved for manual recovery".into());
    }
    Ok(true)
}
fn complete(
    s: &ssh2::Sftp,
    job: &Pending,
    journal: &Journal,
    offset: u64,
    cancel: &AtomicBool,
) -> Result<Receipt, String> {
    if job.upload && backup_valid(s, job, cancel)? {
        s.unlink(&backup(job)).map_err(
            |_| "Transfer committed; original backup retained until Resume finishes cleanup",
        )?;
    }
    journal.remove()?;
    Ok(Receipt {
        id: job.id.clone(),
        sha256: job.sha256.clone(),
        bytes: job.source.size,
        resumed_from: offset,
    })
}
fn execute(
    p: &Profile,
    mut job: Pending,
    journal: Journal,
    resuming: bool,
    cancel: &AtomicBool,
    progress: &AtomicU64,
) -> Result<Receipt, String> {
    let stage = std::cell::Cell::new("connection");
    let result = (|| {
        stopped(cancel)?;
        let session = files::connect(p)?;
        original_host(&session, &job)?;
        let sftp = session.sftp().map_err(files::error)?;
        files::remote_path(&sftp, &job.remote, job.upload)?;
        if job.phase == "commit" {
            let dest = destination(&sftp, &job, cancel)?;
            if dest
                .as_ref()
                .is_some_and(|(stamp, hash)| stamp.size == job.source.size && hash == &job.sha256)
            {
                // A lost reply after rename is completion, not another overwrite.
                let missing = if job.upload {
                    matches!(sftp.lstat(&part(&job)),Err(e)if e.code()==ssh2::ErrorCode::SFTP(2))
                } else {
                    matches!(fs::symlink_metadata(part(&job)),Err(e)if e.kind()==std::io::ErrorKind::NotFound)
                };
                if missing {
                    progress.store(job.source.size, Ordering::Relaxed);
                    return complete(&sftp, &job, &journal, job.source.size, cancel);
                }
                if !job.upload {
                    let a = fs::symlink_metadata(part(&job)).map_err(files::error)?;
                    let b = fs::symlink_metadata(&job.local).map_err(files::error)?;
                    if a.is_file() && a.dev() == b.dev() && a.ino() == b.ino() && a.nlink() == 2 {
                        fs::remove_file(part(&job)).map_err(files::error)?;
                        progress.store(job.source.size, Ordering::Relaxed);
                        return complete(&sftp, &job, &journal, job.source.size, cancel);
                    }
                }
            }
            if job.upload && dest.is_none() && backup_valid(&sftp, &job, cancel)? {
                sftp.rename(&backup(&job), &job.remote, Some(RenameFlags::empty()))
                    .map_err(|_| "Original backup preserved; reconnect and Resume to recover")?;
            }
        }
        stage.set("source verification");
        let (mut input, stamp) = source(&sftp, &job)?;
        if stamp != job.source {
            return Err("Source metadata changed; discard the partial and start again".into());
        }
        if resuming {
            if full_hash(&mut input, cancel)? != job.sha256 {
                return Err("Source bytes changed; partial preserved, start a new transfer".into());
            }
            input.seek(SeekFrom::Start(0)).map_err(files::error)?;
        }
        stage.set("partial open");
        let (mut output, offset) = partial(&sftp, &mut job, &journal)?;
        stage.set("partial prefix verification");
        let (mut digest, n) = hash(&mut input, offset, cancel)?;
        let (part_hash, m) = hash(&mut output, offset, cancel)?;
        if n != offset || m != offset || digest.clone().finalize() != part_hash.finalize() {
            return Err(
                "Partial prefix does not match the source; preserved, discard or inspect it".into(),
            );
        }
        // Clear libssh2 read-ahead state before changing a read/write handle's direction.
        input.seek(SeekFrom::Start(offset)).map_err(files::error)?;
        output.seek(SeekFrom::Start(offset)).map_err(files::error)?;
        stage.set("copy");
        progress.store(offset, Ordering::Relaxed);
        let mut count = offset;
        let mut bytes = [0u8; 65536];
        while count < job.source.size {
            stopped(cancel)?;
            let n = input
                .read(&mut bytes[..((job.source.size - count).min(65536)) as usize])
                .map_err(files::error)?;
            if n == 0 {
                return Err("Source shortened during transfer; partial preserved".into());
            }
            output.write_all(&bytes[..n]).map_err(files::error)?;
            digest.update(&bytes[..n]);
            count += n as u64;
            progress.store(count, Ordering::Relaxed);
        }
        stage.set("partial flush/close");
        output.finish()?;
        let (_, after) = source(&sftp, &job)?;
        if after != job.source || format!("{:x}", digest.finalize()) != job.sha256 {
            return Err("Source changed during transfer; partial preserved".into());
        }
        stage.set("complete partial verification");
        let verified = if job.upload {
            full_hash(&mut sftp.open(&part(&job)).map_err(files::error)?, cancel)?
        } else {
            full_hash(&mut regular(&part(&job), false)?, cancel)?
        };
        if verified != job.sha256 {
            return Err("Partial SHA-256 mismatch; destination unchanged".into());
        }
        stage.set("destination review check");
        if destination(&sftp, &job, cancel)? != job.destination {
            return Err(
                "Destination changed since overwrite review; preserved, start a new transfer"
                    .into(),
            );
        }
        stopped(cancel)?;
        stage.set("commit");
        job.phase = "commit".into();
        journal.save(&job)?;
        if job.upload {
            if job.destination.is_some() {
                if backup_valid(&sftp, &job, cancel)? {
                    return Err(
                        "An existing original backup is preserved; inspect it before continuing"
                            .into(),
                    );
                }
                sftp.rename(&job.remote, &backup(&job), Some(RenameFlags::empty()))
                    .map_err(files::error)?;
            }
            if sftp
                .rename(&part(&job), &job.remote, Some(RenameFlags::empty()))
                .is_err()
            {
                if job.destination.is_some() {
                    let _ = sftp.rename(&backup(&job), &job.remote, Some(RenameFlags::empty()));
                }
                return Err(
                    "Commit interrupted; Resume recovers the verified partial/original backup"
                        .into(),
                );
            }
        } else if job.overwrite {
            fs::rename(part(&job), &job.local).map_err(files::error)?;
        } else {
            fs::hard_link(part(&job), &job.local).map_err(files::error)?;
            fs::remove_file(part(&job)).map_err(files::error)?;
        }
        complete(&sftp, &job, &journal, offset, cancel)
    })();
    result.map_err(|e| {
        format!(
            "Transfer {} paused during {}: {e}. Resume or discard it in Pending transfers.",
            job.id,
            stage.get()
        )
    })
}
pub fn discard(p: &Profile, id: &str, cancel: &AtomicBool) -> Result<(), String> {
    let journal = Journal::open(id)?;
    let job = journal.load(p)?;
    stopped(cancel)?;
    if job.upload {
        let session = files::connect(p)?;
        original_host(&session, &job)?;
        let sftp = session.sftp().map_err(files::error)?;
        files::remote_path(
            &sftp,
            job.remote.parent().ok_or("Remote parent unavailable")?,
            false,
        )?;
        if backup_valid(&sftp, &job, cancel)? {
            let dest = destination(&sftp, &job, cancel)?;
            if dest.is_none() {
                sftp.rename(&backup(&job), &job.remote, Some(RenameFlags::empty()))
                    .map_err(files::error)?;
            } else if dest
                .as_ref()
                .is_some_and(|(stamp, hash)| stamp.size == job.source.size && hash == &job.sha256)
            {
                sftp.unlink(&backup(&job)).map_err(files::error)?;
            } else {
                return Err(
                    "Original backup and changed destination preserved; inspect before discarding"
                        .into(),
                );
            }
        }
        match sftp.lstat(&part(&job)) {
            Err(e) if e.code() == ssh2::ErrorCode::SFTP(2) => (),
            Err(e) => return Err(files::error(e)),
            Ok(_) => {
                let stamp = remote_stamp(&sftp, &part(&job))?;
                if stamp.mode != 0o600 || job.part_uid.is_some_and(|uid| uid != stamp.uid) {
                    return Err("Changed partial ownership preserved".into());
                }
                sftp.unlink(&part(&job)).map_err(files::error)?;
            }
        }
    } else {
        match fs::symlink_metadata(part(&job)) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(files::error(e)),
            Ok(_) => {
                let f = regular(&part(&job), false)?;
                if !private_file(&f) {
                    return Err("Changed local partial ownership preserved".into());
                }
                fs::remove_file(part(&job)).map_err(files::error)?;
            }
        }
    }
    journal.remove()
}
