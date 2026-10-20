//! Whether a host already accepts one of this user's existing SSH public keys.
//! Runs only on explicit request. Batch mode never prompts, offers no
//! password, accepts no new or changed host key and forwards no agent.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    fs, io,
    io::{Read, Seek, SeekFrom},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{fs::PermissionsExt, process::CommandExt},
    },
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Access {
    /// Public-key login succeeded.
    Verified,
    /// The server offers public-key login but none of the offered keys was
    /// accepted. Not proof that a key is absent from authorized_keys.
    NotVerified,
    /// Host key not in known_hosts yet; never accepted automatically.
    UnknownHostTrust,
    /// Host key differs from known_hosts.
    HostKeyChanged,
    /// The server only offers password or keyboard-interactive login.
    NeedsInteraction,
    Unreachable,
}
impl Access {
    pub fn describe(self) -> &'static str {
        match self {
            Self::Verified => "Verified key login",
            Self::NotVerified => "Not verified",
            Self::UnknownHostTrust => {
                "Unknown host trust — connect once in a terminal to verify the host key"
            }
            Self::HostKeyChanged => "Host key changed — check before connecting",
            Self::NeedsInteraction => "Needs interaction",
            Self::Unreachable => "Unreachable",
        }
    }
}

/// Batch options come first: for each option ssh keeps the first value.
const BATCH: &[&str] = &[
    "BatchMode=yes",
    "ControlMaster=no",
    "ControlPath=none",
    "ControlPersist=no",
    "PasswordAuthentication=no",
    "KbdInteractiveAuthentication=no",
    "PreferredAuthentications=publickey",
    "PubkeyAuthentication=yes",
    "HostbasedAuthentication=no",
    "GSSAPIAuthentication=no",
    "StrictHostKeyChecking=yes",
    "CheckHostIP=no",
    "VerifyHostKeyDNS=no",
    "UpdateHostKeys=no",
    "AddKeysToAgent=no",
    "ConnectTimeout=6",
    "ForwardAgent=no",
    "ClearAllForwardings=yes",
    "ForwardX11=no",
    "RequestTTY=no",
    "PermitLocalCommand=no",
    "NumberOfPasswordPrompts=0",
    // ssh reports the key the server accepted ("Server accepts key") at this
    // level, so one connection identifies it without extra failed attempts.
    "LogLevel=DEBUG1",
];

/// The profile's ssh argv with batch options and `extra` before it, then `true`.
pub fn probe_argv(argv: &[String], extra: &[String]) -> Vec<String> {
    let mut out = vec![argv.first().cloned().unwrap_or_else(|| "ssh".into())];
    for &option in BATCH {
        out.extend(["-o".into(), option.into()]);
    }
    out.extend(extra.iter().cloned());
    out.extend(argv.iter().skip(1).cloned());
    out.push("true".into());
    out
}

pub fn classify(success: bool, stderr: &str) -> Access {
    if success {
        return Access::Verified;
    }
    let e = stderr.to_lowercase();
    if e.contains("remote host identification has changed") {
        Access::HostKeyChanged
    } else if e.contains("host key verification failed")
        || (e.contains("no ") && e.contains("host key is known"))
    {
        Access::UnknownHostTrust
    } else if let Some(methods) = e
        .split("permission denied (")
        .nth(1)
        .and_then(|rest| rest.split(')').next())
    {
        if methods.split(',').any(|m| m.trim() == "publickey") {
            Access::NotVerified
        } else {
            Access::NeedsInteraction
        }
    } else if e.contains("too many authentication failures") {
        Access::NotVerified
    } else {
        Access::Unreachable
    }
}

fn run_input(
    argv: &[String],
    input: &[u8],
    allowed: &AtomicBool,
    timeout: Duration,
) -> Option<(bool, String, String)> {
    let mut command = Command::new(argv.first()?);
    command
        .args(&argv[1..])
        .env("SSH_ASKPASS_REQUIRE", "never")
        .env_remove("SSH_ASKPASS");
    let output = crate::remote_paste_upload::bounded_process::capture(
        &mut command,
        input,
        allowed,
        timeout,
        256 * 1024,
        64 * 1024,
    )
    .ok()?;
    Some((
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    ))
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Target {
    pub host: String,
    pub port: u16,
    pub account: String,
    pub route: Vec<String>,
}

pub(crate) struct Plan {
    pub(crate) argv: Vec<String>,
    target: Target,
    configurations: Vec<String>,
}

fn profile_argv(profile: &crate::profiles::Profile) -> Result<Vec<String>, String> {
    let (host, port) = crate::profiles::endpoint(&profile.server, 0)?;
    let mut argv = vec!["ssh".to_owned()];
    if port != 0 {
        argv.extend(["-p".into(), port.to_string()]);
    }
    if !profile.username.is_empty() {
        argv.extend(["-l".into(), profile.username.clone()]);
    }
    if !profile.preferences.identity_file.is_empty() {
        argv.extend([
            "-i".into(),
            profile.preferences.identity_file.clone(),
            "-o".into(),
            "IdentitiesOnly=yes".into(),
        ]);
    }
    if !profile.preferences.known_hosts.is_empty() {
        let file = profile
            .preferences
            .known_hosts
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        argv.extend(["-o".into(), format!("UserKnownHostsFile=\"{file}\"")]);
    }
    if !profile.preferences.ssh_jump_alias.is_empty() {
        crate::ssh_jump::validate(&profile.preferences.ssh_jump_alias)?;
        argv.extend(["-J".into(), profile.preferences.ssh_jump_alias.clone()]);
    }
    argv.extend(["--".into(), host]);
    Ok(argv)
}

fn config(argv: &[String], allowed: &AtomicBool, deadline: Instant) -> Result<String, String> {
    let timeout = deadline
        .saturating_duration_since(Instant::now())
        .min(Duration::from_secs(5));
    if timeout.is_zero() {
        return Err("SSH configuration deadline reached".into());
    }
    let query = probe_argv(argv, &["-G".into()]);
    let Some((true, output, _)) = run_input(&query, &[], allowed, timeout) else {
        return Err("OpenSSH configuration could not be resolved within its bound".into());
    };
    Ok(output)
}

fn fields(config: &str) -> BTreeMap<&str, Vec<&str>> {
    let mut fields: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for line in config.lines() {
        if let Some((name, value)) = line.split_once(' ') {
            fields.entry(name).or_default().push(value);
        }
    }
    fields
}
fn field<'a>(config: &'a BTreeMap<&str, Vec<&str>>, name: &str) -> Option<&'a str> {
    config.get(name)?.first().copied().filter(|v| *v != "none")
}

fn jump_argv(jump: &str) -> Result<Vec<String>, String> {
    if jump.len() > 512 || jump.chars().any(char::is_control) {
        return Err("Invalid configured SSH jump".into());
    }
    let (user, server) = jump
        .rsplit_once('@')
        .map_or((None, jump), |(user, server)| (Some(user), server));
    // OpenSSH -G brackets even IPv4/DNS jump hosts when a port is present.
    // The actual ssh destination argument must be the unbracketed host.
    let (host, port) = if let Some(bracketed) = server.strip_prefix('[') {
        let (host, suffix) = bracketed
            .split_once(']')
            .ok_or("Invalid bracketed SSH jump")?;
        if host.is_empty() || host.starts_with('-') {
            return Err("Invalid SSH jump host".into());
        }
        let port = if suffix.is_empty() {
            0
        } else {
            suffix
                .strip_prefix(':')
                .and_then(|p| p.parse::<u16>().ok())
                .filter(|p| *p != 0)
                .ok_or("Invalid SSH jump port")?
        };
        (host.to_owned(), port)
    } else {
        crate::profiles::endpoint(server, 0)?
    };
    let mut argv = vec!["ssh".into()];
    if let Some(user) = user {
        argv.extend(["-l".into(), user.into()]);
    }
    if port != 0 {
        argv.extend(["-p".into(), port.to_string()]);
    }
    argv.extend(["--".into(), host]);
    Ok(argv)
}

fn proxy_command(argv: &[String]) -> String {
    // OpenSSH expands percent tokens before invoking its shell, even inside quotes.
    argv.iter()
        .map(|s| format!("'{}'", s.replace('%', "%%").replace('\'', "'\\''")))
        .collect::<Vec<_>>()
        .join(" ")
}

fn plan_node(
    mut argv: Vec<String>,
    override_jumps: Option<Vec<String>>,
    log: Option<&str>,
    allowed: &AtomicBool,
    deadline: Instant,
    depth: usize,
) -> Result<Plan, String> {
    if depth > 8 {
        return Err("SSH jump chain exceeds eight hops or contains a cycle".into());
    }
    let output = config(&argv, allowed, deadline)?;
    let values = fields(&output);
    let mut target = Target {
        host: field(&values, "hostname")
            .ok_or("SSH target host unavailable")?
            .into(),
        port: field(&values, "port")
            .and_then(|p| p.parse().ok())
            .filter(|p| *p != 0)
            .ok_or("SSH target port unavailable")?,
        account: field(&values, "user")
            .ok_or("SSH target account unavailable")?
            .into(),
        route: Vec::new(),
    };
    if field(&values, "proxycommand").is_some() && override_jumps.is_none() {
        return Err("Custom ProxyCommand requires an explicit terminal check; it cannot guarantee the key-check policy for every SSH hop".into());
    }
    let jumps = override_jumps.unwrap_or_else(|| {
        field(&values, "proxyjump")
            .map(|s| s.split(',').map(str::to_owned).collect())
            .unwrap_or_default()
    });
    let mut configurations = vec![output.clone()];
    if let Some(last) = jumps.last() {
        let previous = (jumps.len() > 1).then(|| jumps[..jumps.len() - 1].to_vec());
        let hop = plan_node(
            jump_argv(last)?,
            previous,
            log,
            allowed,
            deadline,
            depth + 1,
        )?;
        target.route = hop.target.route.clone();
        target.route.push(format!(
            "{}@{}:{}",
            hop.target.account, hop.target.host, hop.target.port
        ));
        configurations.extend(hop.configurations);
        let mut extra = vec!["-W".into(), format!("[{}]:{}", target.host, target.port)];
        if let Some(log) = log {
            extra.extend(["-E".into(), log.into()]);
        }
        let mut command = probe_argv(&hop.argv, &extra);
        command.pop(); // Forwarding hop has no remote command.
        command.splice(1..1, ["-o".into(), "LogLevel=ERROR".into()]);
        let mut without_jump = vec![argv[0].clone()];
        let mut i = 1;
        while i < argv.len() {
            if argv[i] == "-J" {
                i += 2;
            } else {
                without_jump.push(argv[i].clone());
                i += 1;
            }
        }
        argv = vec![
            "ssh".into(),
            "-o".into(),
            format!("ProxyCommand={}", proxy_command(&command)),
        ];
        argv.extend(without_jump.into_iter().skip(1));
    }
    Ok(Plan {
        argv,
        target,
        configurations,
    })
}

pub(crate) fn plan(
    profile: &crate::profiles::Profile,
    log: Option<&str>,
    allowed: &AtomicBool,
) -> Result<Plan, String> {
    plan_node(
        profile_argv(profile)?,
        None,
        log,
        allowed,
        Instant::now() + Duration::from_secs(8),
        0,
    )
}

/// A public identity this user can offer: agent key or key file with `.pub`.
#[derive(Clone, Debug, PartialEq)]
pub struct Identity {
    /// `SHA256:…` fingerprint and comment, as ssh-keygen prints them.
    pub fingerprint: String,
    pub public_key: String,
}

/// Agent identities (`ssh-add -L`) and ~/.ssh public keys; private keys are never read.
pub fn identities() -> Vec<Identity> {
    identities_cancel(&AtomicBool::new(true))
}

pub fn identities_cancel(allowed: &AtomicBool) -> Vec<Identity> {
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut lines: Vec<String> = Vec::new();
    if let Some((true, out, _)) = run_input(
        &["ssh-add".into(), "-L".into()],
        &[],
        allowed,
        Duration::from_secs(3),
    ) {
        lines.extend(out.lines().map(str::to_owned));
    }
    for key in crate::ssh_keys::detect() {
        let mut public = key.path.as_os_str().to_owned();
        public.push(".pub");
        if !allowed.load(std::sync::atomic::Ordering::Acquire) || Instant::now() >= deadline {
            break;
        }
        if let Ok(bytes) = crate::credentials::bounded_file(&PathBuf::from(public), 16 * 1024) {
            if let Ok(text) = std::str::from_utf8(&bytes) {
                lines.extend(text.lines().take(1).map(str::to_owned));
            }
        }
    }
    let mut seen = Vec::new();
    lines
        .into_iter()
        .filter_map(|line| {
            if !allowed.load(std::sync::atomic::Ordering::Acquire) || Instant::now() >= deadline {
                return None;
            }
            let blob = line.split_whitespace().nth(1)?.to_owned();
            if seen.contains(&blob) {
                return None;
            }
            seen.push(blob);
            Some(Identity {
                fingerprint: fingerprint(&line, allowed)
                    .unwrap_or_else(|| "unknown fingerprint".into()),
                public_key: line,
            })
        })
        .take(64)
        .collect()
}

fn fingerprint(public_key: &str, allowed: &AtomicBool) -> Option<String> {
    let (ok, output, _) = run_input(
        &[
            "ssh-keygen".into(),
            "-l".into(),
            "-E".into(),
            "sha256".into(),
            "-f".into(),
            "-".into(),
        ],
        public_key.as_bytes(),
        allowed,
        Duration::from_secs(2),
    )?;
    if !ok {
        return None;
    }
    let mut parts = output.split_whitespace();
    parts.next()?;
    Some(parts.collect::<Vec<_>>().join(" "))
}

#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    pub access: Access,
    /// Fingerprint of the accepted key when a one-key attempt proved it.
    pub accepted_key: Option<String>,
    pub reason: Option<String>,
}
impl Outcome {
    pub fn describe(&self) -> String {
        if let Some(reason) = &self.reason {
            return format!("{} · {reason}", self.access.describe());
        }
        match (&self.access, &self.accepted_key) {
            (Access::Verified, Some(key)) => format!("Verified key login · {key}"),
            (Access::Verified, None) => "SSH public-key login verified; exact key unknown".into(),
            (access, _) => access.describe().into(),
        }
    }
}

/// Check one endpoint and identify its accepted key from the separate client
/// diagnostic stream, after completed public-key authentication.
pub fn probe(profile: &crate::profiles::Profile, keys: &[Identity]) -> Outcome {
    probe_cancel(profile, keys, &AtomicBool::new(true))
}

pub fn probe_cancel(
    profile: &crate::profiles::Profile,
    keys: &[Identity],
    allowed: &AtomicBool,
) -> Outcome {
    let unreachable = Outcome {
        access: Access::Unreachable,
        accepted_key: None,
        reason: None,
    };
    let Ok(mut log) = client_log() else {
        return unreachable;
    };
    let log_path = format!("/proc/{}/fd/{}", std::process::id(), log.as_raw_fd());
    let plan = match plan(profile, Some(&log_path), allowed) {
        Ok(plan) => plan,
        Err(reason) => {
            return Outcome {
                access: Access::NeedsInteraction,
                accepted_key: None,
                reason: Some(reason),
            };
        }
    };
    let extra = vec!["-E".into(), log_path];
    let argv = probe_argv(&plan.argv, &extra);
    let mut command = Command::new(&argv[0]);
    command
        .args(&argv[1..])
        .env("SSH_ASKPASS_REQUIRE", "never")
        .env_remove("SSH_ASKPASS");
    // OpenSSH owns this in-memory log. Remote banners and command stderr use
    // the separate bounded stderr pipe and cannot identify an accepted key.
    unsafe {
        command.pre_exec(|| {
            let cap = libc::rlimit {
                rlim_cur: 262144,
                rlim_max: 262144,
            };
            if libc::setrlimit(libc::RLIMIT_FSIZE, &cap) != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let Ok(output) = crate::remote_paste_upload::bounded_process::capture(
        &mut command,
        &[],
        allowed,
        Duration::from_secs(15),
        256 * 1024,
        64 * 1024,
    ) else {
        return unreachable;
    };
    if log.seek(SeekFrom::Start(0)).is_err() {
        return unreachable;
    }
    let mut bytes = Vec::new();
    if log.take(262145).read_to_end(&mut bytes).is_err() || bytes.len() > 262144 {
        return unreachable;
    }
    let diagnostics = String::from_utf8_lossy(&bytes);
    let public_key_success = diagnostics.lines().any(authenticated_public_key);
    let access = if output.status.success() && !public_key_success {
        Access::NotVerified
    } else if !output.status.success() && diagnostics.contains("with partial success") {
        Access::NeedsInteraction
    } else {
        classify(output.status.success(), &diagnostics)
    };
    let accepted_key = if access == Access::Verified {
        accepted_fingerprint(&diagnostics).map(|fp| {
            keys.iter()
                .find(|k| k.fingerprint.split_whitespace().next() == Some(fp.as_str()))
                .map(|k| k.fingerprint.clone())
                .unwrap_or(fp)
        })
    } else {
        None
    };
    Outcome {
        access,
        accepted_key,
        reason: None,
    }
}

fn client_log() -> io::Result<fs::File> {
    let fd = unsafe { libc::memfd_create(c"ruwac-ssh-check".as_ptr(), libc::MFD_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let file = unsafe { fs::File::from_raw_fd(fd) };
    file.set_permissions(fs::Permissions::from_mode(0o600))?;
    Ok(file)
}

fn authenticated_public_key(line: &str) -> bool {
    line.starts_with("Authenticated to ") && line.ends_with(" using \"publickey\".")
}

/// The `SHA256:` fingerprint from ssh's "Server accepts key" debug line.
fn accepted_fingerprint(stderr: &str) -> Option<String> {
    let before_auth: Vec<_> = stderr
        .lines()
        .take_while(|line| !authenticated_public_key(line))
        .collect();
    before_auth
        .into_iter()
        .rev()
        .filter(|l| l.starts_with("debug1: Server accepts key:"))
        .flat_map(str::split_whitespace)
        .find(|w| w.starts_with("SHA256:"))
        .map(str::to_owned)
}

/// Everything a result depends on: OpenSSH's resolved configuration for this
/// endpoint (HostName, User, Port, ProxyJump, HostKeyAlias, IdentityFile…),
/// the known_hosts files it names, and the offered public keys.
pub fn inputs(profile: &crate::profiles::Profile, keys: &[Identity]) -> String {
    inspect(profile, keys, &AtomicBool::new(true))
        .map(|(hash, _)| hash)
        .unwrap_or_default()
}

fn expanded(path: &str, target: &Target) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    let local = gtk::glib::user_name().to_string_lossy().into_owned();
    let path = path.trim_matches('"');
    let path = path
        .strip_prefix("~/")
        .map_or(path.to_owned(), |p| format!("{home}/{p}"));
    let mut result = String::new();
    let mut chars = path.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            result.push(c);
            continue;
        }
        match chars.next() {
            Some('d') => result.push_str(&home),
            Some('u') => result.push_str(&local),
            Some('h') => result.push_str(&target.host),
            Some('p') => result.push_str(&target.port.to_string()),
            Some('r') => result.push_str(&target.account),
            Some('%') => result.push('%'),
            Some(c) => {
                result.push('%');
                result.push(c);
            }
            None => result.push('%'),
        }
    }
    PathBuf::from(result)
}

fn metadata(hash: &mut Sha256, path: &Path, contents: bool) {
    use std::os::unix::{ffi::OsStrExt, fs::MetadataExt};
    hash.update(path.as_os_str().as_bytes());
    match fs::metadata(path) {
        Ok(m) => {
            hash.update(
                format!(
                    "{}:{}:{}:{}:{}:{}:{}:{}:{}",
                    m.dev(),
                    m.ino(),
                    m.len(),
                    m.mtime(),
                    m.mtime_nsec(),
                    m.ctime(),
                    m.ctime_nsec(),
                    m.mode(),
                    m.uid()
                )
                .as_bytes(),
            );
            if contents {
                if let Ok(bytes) = crate::credentials::bounded_file(path, 8 << 20) {
                    hash.update(bytes);
                }
            }
        }
        Err(_) => hash.update(b"missing-or-unreadable"),
    }
}

/// Resolve only after the explicit check. Snapshot every hop, both trust-file
/// families, identity metadata/public halves and the actual configured agent.
/// Private identity-file contents are never opened.
pub fn inspect(
    profile: &crate::profiles::Profile,
    keys: &[Identity],
    allowed: &AtomicBool,
) -> Option<(String, Target)> {
    let plan = plan(profile, None, allowed).ok()?;
    let mut hash = Sha256::new();
    hash.update(b"ruwac-explicit-key-check-v2\0");
    for config in &plan.configurations {
        hash.update(config.as_bytes());
        let values = fields(config);
        let target = Target {
            host: field(&values, "hostname")?.into(),
            account: field(&values, "user")?.into(),
            port: field(&values, "port")?.parse().ok()?,
            route: Vec::new(),
        };
        for name in ["userknownhostsfile", "globalknownhostsfile"] {
            for files in values.get(name).into_iter().flatten() {
                // ssh -G may print unquoted paths containing spaces. Include
                // every existing contiguous candidate, rather than silently
                // omitting the real configured trust file.
                let words: Vec<_> = files.split_whitespace().collect();
                if words.len() > 32 {
                    return None;
                }
                for start in 0..words.len() {
                    for end in start + 1..=words.len() {
                        let path = expanded(&words[start..end].join(" "), &target);
                        if end == start + 1 || path.is_file() {
                            metadata(&mut hash, &path, true);
                        }
                    }
                }
            }
        }
        for name in ["identityfile", "certificatefile", "securitykeyprovider"] {
            for file in values.get(name).into_iter().flatten() {
                let path = expanded(file, &target);
                metadata(&mut hash, &path, name != "identityfile");
                if name == "identityfile" {
                    let mut public = path.as_os_str().to_owned();
                    public.push(".pub");
                    metadata(&mut hash, &PathBuf::from(public), true);
                }
            }
        }
        if field(&values, "knownhostscommand").is_some() {
            // Dynamic trust-command output cannot be reused as a file snapshot.
            hash.update(ruwac_protocol::id("uncacheable-trust"));
        }
        let configured_agent = values.get("identityagent").and_then(|v| v.first()).copied();
        let agent = match configured_agent {
            Some("none") => None,
            None | Some("SSH_AUTH_SOCK") => std::env::var_os("SSH_AUTH_SOCK").map(PathBuf::from),
            Some(path) => Some(expanded(path, &target)),
        };
        if let Some(agent) = agent {
            metadata(&mut hash, &agent, false);
            let mut command = Command::new("ssh-add");
            command
                .arg("-L")
                .env("SSH_AUTH_SOCK", agent)
                .env("SSH_ASKPASS_REQUIRE", "never");
            if let Ok(output) = crate::remote_paste_upload::bounded_process::capture(
                &mut command,
                &[],
                allowed,
                Duration::from_secs(3),
                256 * 1024,
                4096,
            ) {
                hash.update(output.stdout);
            }
        }
    }
    for key in keys {
        hash.update(key.public_key.as_bytes());
    }
    Some((format!("{:x}", hash.finalize()), plan.target))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Checked {
    pub access: Access,
    #[serde(default)]
    pub accepted_key: Option<String>,
    /// Seconds since the Unix epoch.
    pub checked: u64,
    /// Hash of `inputs()`; a different value means the result is stale.
    #[serde(default)]
    pub inputs: String,
    #[serde(default)]
    pub source_id: String,
    #[serde(default)]
    pub target: Option<Target>,
    #[serde(default)]
    pub reason: Option<String>,
}

fn cache_path() -> io::Result<PathBuf> {
    use std::os::unix::fs::MetadataExt;
    let dir = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .ok_or_else(|| io::Error::other("configuration directory unavailable"))?
        .join("RUWAC");
    // The desktop creates this private directory at startup.
    let m = fs::symlink_metadata(&dir)?;
    if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o777 != 0o700 {
        return Err(io::Error::other("unsafe configuration directory"));
    }
    Ok(dir.join("ssh-access.json"))
}
/// Earlier results by endpoint ID; nonsecret and bounded. Results from the
/// previous format (no inputs hash) are treated as stale.
pub fn load() -> HashMap<String, Checked> {
    cache_path()
        .ok()
        .and_then(|p| crate::credentials::bounded_file(&p, 2 * 1024 * 1024).ok())
        .and_then(|b| serde_json::from_slice::<HashMap<String, Checked>>(&b).ok())
        .map(|m| {
            m.into_iter()
                .filter(|(_, c)| {
                    !c.inputs.is_empty() && !c.source_id.is_empty() && c.target.is_some()
                })
                .collect()
        })
        .unwrap_or_default()
}
pub fn save(cache: &HashMap<String, Checked>) -> io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let path = cache_path()?;
    let mut entries: Vec<_> = cache.iter().collect();
    entries.sort_by_key(|(id, c)| (std::cmp::Reverse(c.checked), *id));
    let bounded: HashMap<_, _> = entries.into_iter().take(1000).collect();
    let bytes = serde_json::to_vec(&bounded)?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err(io::Error::other("SSH result cache exceeds size limit"));
    }
    let temporary = path.with_extension(ruwac_protocol::id("tmp"));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&temporary)?;
        io::Write::write_all(&mut file, &bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolved_jump_brackets_are_not_sent_as_part_of_the_hostname() {
        for (jump, host, port) in [
            ("user@[127.0.0.1]:2200", "127.0.0.1", "2200"),
            ("user@[::1]:2201", "::1", "2201"),
            ("user@[generated-alias]:2202", "generated-alias", "2202"),
        ] {
            let argv = jump_argv(jump).unwrap();
            assert_eq!(argv, vec!["ssh", "-l", "user", "-p", port, "--", host]);
        }
        for bad in ["user@[host]:0", "user@[host]trailing", "user@[]:22"] {
            assert!(jump_argv(bad).is_err());
        }
    }
    #[test]
    fn batch_options_precede_profile_options_and_results_are_classified() {
        let argv = probe_argv(
            &[
                "ssh".into(),
                "-o".into(),
                "StrictHostKeyChecking=ask".into(),
                "--".into(),
                "host".into(),
            ],
            &["-i".into(), "k.pub".into()],
        );
        let batch = argv
            .iter()
            .position(|a| a == "StrictHostKeyChecking=yes")
            .unwrap();
        let ask = argv
            .iter()
            .position(|a| a == "StrictHostKeyChecking=ask")
            .unwrap();
        assert!(batch < ask, "ssh keeps the first value given");
        assert!(argv.contains(&"PasswordAuthentication=no".to_string()));
        assert!(argv.contains(&"ControlPath=none".to_string()));
        assert!(
            argv.iter().position(|a| a == "k.pub").unwrap()
                < argv.iter().position(|a| a == "--").unwrap()
        );
        assert_eq!(argv.last().unwrap(), "true");
        assert_eq!(classify(true, ""), Access::Verified);
        assert_eq!(
            classify(false, "u@h: Permission denied (publickey,password)."),
            Access::NotVerified
        );
        assert_eq!(
            classify(
                false,
                "u@h: Permission denied (password,keyboard-interactive)."
            ),
            Access::NeedsInteraction
        );
        assert_eq!(
            classify(
                false,
                "No ED25519 host key is known for h and you have requested strict checking.\r\nHost key verification failed."
            ),
            Access::UnknownHostTrust
        );
        assert_eq!(
            classify(
                false,
                "@ WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED! @\nHost key verification failed."
            ),
            Access::HostKeyChanged
        );
        assert_eq!(
            classify(false, "ssh: connect to host h port 22: Connection refused"),
            Access::Unreachable
        );
        assert_eq!(
            accepted_fingerprint(
                "debug1: Offering public key: k ED25519 SHA256:aaa agent\ndebug1: Server accepts key: /h/.ssh/id ED25519 SHA256:bbb explicit\n"
            )
            .as_deref(),
            Some("SHA256:bbb")
        );
        assert_eq!(accepted_fingerprint("Permission denied (publickey)."), None);
        let unknown = Outcome {
            access: Access::Verified,
            accepted_key: None,
            reason: None,
        };
        assert_eq!(
            unknown.describe(),
            "SSH public-key login verified; exact key unknown"
        );
    }
}
