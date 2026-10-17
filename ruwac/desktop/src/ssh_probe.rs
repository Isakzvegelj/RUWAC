//! Whether a host already accepts one of this user's existing SSH public keys.
//! Runs only on explicit request. Batch mode never prompts, offers no
//! password, accepts no new or changed host key and forwards no agent.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs, io,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
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
            Self::NeedsInteraction => "Needs interaction (password or keyboard-interactive only)",
            Self::Unreachable => "Unreachable",
        }
    }
}

/// Batch options come first: for each option ssh keeps the first value.
const BATCH: [&str; 12] = [
    "BatchMode=yes",
    "ControlMaster=no",
    "ControlPath=none",
    "ControlPersist=no",
    "PasswordAuthentication=no",
    "KbdInteractiveAuthentication=no",
    "StrictHostKeyChecking=yes",
    "UpdateHostKeys=no",
    "ConnectTimeout=6",
    "ForwardAgent=no",
    "ClearAllForwardings=yes",
    // ssh reports the key the server accepted ("Server accepts key") at this
    // level, so one connection identifies it without extra failed attempts.
    "LogLevel=DEBUG1",
];

/// The profile's ssh argv with batch options and `extra` before it, then `true`.
pub fn probe_argv(argv: &[String], extra: &[String]) -> Vec<String> {
    let mut out = vec![argv.first().cloned().unwrap_or_else(|| "ssh".into())];
    for option in BATCH {
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

fn run(argv: &[String], timeout: Duration) -> Option<(bool, String, String)> {
    let mut child = Command::new(&argv[0])
        .args(&argv[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let (mut out, mut err) = (String::new(), String::new());
    if let Some(mut s) = child.stdout.take() {
        let _ = s.by_ref().take(256 * 1024).read_to_string(&mut out);
    }
    if let Some(mut s) = child.stderr.take() {
        let _ = s.by_ref().take(64 * 1024).read_to_string(&mut err);
    }
    Some((status?.success(), out, err))
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
    let mut lines: Vec<String> = Vec::new();
    if let Some((true, out, _)) = run(&["ssh-add".into(), "-L".into()], Duration::from_secs(3)) {
        lines.extend(out.lines().map(str::to_owned));
    }
    for key in crate::ssh_keys::detect() {
        let mut public = key.path.as_os_str().to_owned();
        public.push(".pub");
        if let Ok(text) = fs::read_to_string(PathBuf::from(public)) {
            lines.extend(text.lines().take(1).map(str::to_owned));
        }
    }
    let mut seen = Vec::new();
    lines
        .into_iter()
        .filter_map(|line| {
            let blob = line.split_whitespace().nth(1)?.to_owned();
            if seen.contains(&blob) {
                return None;
            }
            seen.push(blob);
            Some(Identity {
                fingerprint: fingerprint(&line).unwrap_or_else(|| "unknown fingerprint".into()),
                public_key: line,
            })
        })
        .take(8)
        .collect()
}

fn fingerprint(public_key: &str) -> Option<String> {
    let mut child = Command::new("ssh-keygen")
        .args(["-l", "-f", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    io::Write::write_all(child.stdin.as_mut()?, public_key.as_bytes()).ok()?;
    drop(child.stdin.take());
    let out = child.wait_with_output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut parts = text.split_whitespace();
    parts.next()?;
    Some(parts.collect::<Vec<_>>().join(" "))
}

#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    pub access: Access,
    /// Fingerprint of the accepted key when a one-key attempt proved it.
    pub accepted_key: Option<String>,
}
impl Outcome {
    pub fn describe(&self) -> String {
        match (&self.access, &self.accepted_key) {
            (Access::Verified, Some(key)) => format!("Verified key login · {key}"),
            (Access::Verified, None) => "SSH public-key login verified; exact key unknown".into(),
            (access, _) => access.describe().into(),
        }
    }
}

/// Check one endpoint; after a success, identify the accepted key with
/// one-key attempts (agent keys are offered through their public half).
pub fn probe(profile: &crate::profiles::Profile, keys: &[Identity]) -> Outcome {
    let unreachable = Outcome {
        access: Access::Unreachable,
        accepted_key: None,
    };
    let Ok(argv) = crate::profiles::ssh_argv(profile) else {
        return unreachable;
    };
    let Some((ok, _, err)) = run(&probe_argv(&argv, &[]), Duration::from_secs(15)) else {
        return unreachable;
    };
    let access = classify(ok, &err);
    let accepted_key = if access == Access::Verified {
        accepted_fingerprint(&err).map(|fp| {
            keys.iter()
                .find(|k| k.fingerprint.starts_with(&fp))
                .map(|k| k.fingerprint.clone())
                .unwrap_or(fp)
        })
    } else {
        None
    };
    Outcome {
        access,
        accepted_key,
    }
}

/// The `SHA256:` fingerprint from ssh's "Server accepts key" debug line.
fn accepted_fingerprint(stderr: &str) -> Option<String> {
    stderr
        .lines()
        .filter(|l| l.contains("Server accepts key:"))
        .flat_map(str::split_whitespace)
        .find(|w| w.starts_with("SHA256:"))
        .map(str::to_owned)
}

/// Everything a result depends on: OpenSSH's resolved configuration for this
/// endpoint (HostName, User, Port, ProxyJump, HostKeyAlias, IdentityFile…),
/// the known_hosts files it names, and the offered public keys.
pub fn inputs(profile: &crate::profiles::Profile, keys: &[Identity]) -> String {
    let mut hash = Sha256::new();
    if let Ok(mut argv) = crate::profiles::ssh_argv(profile) {
        argv.insert(1, "-G".into());
        if let Some((_, config, _)) = run(&argv, Duration::from_secs(5)) {
            hash.update(config.as_bytes());
            for line in config.lines() {
                if let Some(files) = line.strip_prefix("userknownhostsfile ") {
                    for file in files.split_whitespace() {
                        let path = file.strip_prefix("~/").map_or(PathBuf::from(file), |rest| {
                            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(rest)
                        });
                        if let Ok(bytes) =
                            crate::credentials::bounded_file(Path::new(&path), 8 << 20)
                        {
                            hash.update(&bytes);
                        }
                    }
                }
            }
        }
    }
    for key in keys {
        hash.update(key.public_key.as_bytes());
    }
    format!("{:x}", hash.finalize())
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
        .and_then(|p| crate::credentials::bounded_file(&p, 256 * 1024).ok())
        .and_then(|b| serde_json::from_slice::<HashMap<String, Checked>>(&b).ok())
        .map(|m| {
            m.into_iter()
                .filter(|(_, c)| !c.inputs.is_empty())
                .collect()
        })
        .unwrap_or_default()
}
pub fn save(cache: &HashMap<String, Checked>) -> io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let path = cache_path()?;
    let temporary = path.with_extension(ruwac_protocol::id("tmp"));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&temporary)?;
    io::Write::write_all(&mut file, &serde_json::to_vec(cache)?)?;
    fs::rename(temporary, path)
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
        };
        assert_eq!(
            unknown.describe(),
            "SSH public-key login verified; exact key unknown"
        );
    }
}
