//! Non-interactive check whether a host already accepts this user's SSH key.
//! Batch mode never prompts and offers only public keys (agent, ~/.ssh,
//! ~/.ssh/config, or the endpoint's identity file); no password is sent.
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs, io,
    io::Read,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Access {
    /// Key login works.
    Key,
    /// Host reachable but no offered key is accepted.
    NoKey,
    /// Host key not in known_hosts yet; connect once interactively to trust it.
    HostUnknown,
    Unreachable,
}
impl Access {
    pub fn describe(self) -> &'static str {
        match self {
            Self::Key => "SSH key login works",
            Self::NoKey => "Host reachable; your key is not installed there",
            Self::HostUnknown => "Host key not trusted yet; open a terminal once to verify it",
            Self::Unreachable => "SSH not reachable",
        }
    }
}

/// The profile's ssh argv with batch options (first value wins in ssh) and `true`.
pub fn probe_argv(argv: &[String]) -> Vec<String> {
    let mut out = vec![argv.first().cloned().unwrap_or_else(|| "ssh".into())];
    for option in [
        "BatchMode=yes",
        "PasswordAuthentication=no",
        "KbdInteractiveAuthentication=no",
        "StrictHostKeyChecking=yes",
        "ConnectTimeout=6",
        "ForwardAgent=no",
        "ClearAllForwardings=yes",
    ] {
        out.extend(["-o".into(), option.into()]);
    }
    out.extend(argv.iter().skip(1).cloned());
    out.push("true".into());
    out
}

pub fn classify(success: bool, stderr: &str) -> Access {
    if success {
        return Access::Key;
    }
    let e = stderr.to_lowercase();
    if e.contains("host key verification failed")
        || (e.contains("no ") && e.contains("host key is known"))
    {
        Access::HostUnknown
    } else if e.contains("permission denied") || e.contains("too many authentication failures") {
        Access::NoKey
    } else {
        Access::Unreachable
    }
}

pub fn probe(profile: &crate::profiles::Profile) -> Access {
    let Ok(argv) = crate::profiles::ssh_argv(profile) else {
        return Access::Unreachable;
    };
    let argv = probe_argv(&argv);
    let child = Command::new(&argv[0])
        .args(&argv[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn();
    let Ok(mut child) = child else {
        return Access::Unreachable;
    };
    let deadline = Instant::now() + Duration::from_secs(15);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(100)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.by_ref().take(64 * 1024).read_to_string(&mut stderr);
    }
    match status {
        Some(status) => classify(status.success(), &stderr),
        None => Access::Unreachable,
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Checked {
    pub access: Access,
    /// Seconds since the Unix epoch.
    pub checked: u64,
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
/// Per-machine results of earlier checks, so hosts are probed once, not on every start.
pub fn load() -> HashMap<String, Checked> {
    cache_path()
        .ok()
        .and_then(|p| crate::credentials::bounded_file(&p, 256 * 1024).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
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
    fn batch_options_precede_profile_options_and_classify_results() {
        let argv = probe_argv(&[
            "ssh".into(),
            "-p".into(),
            "22".into(),
            "-o".into(),
            "StrictHostKeyChecking=ask".into(),
            "--".into(),
            "host".into(),
        ]);
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
        assert_eq!(argv.last().unwrap(), "true");
        assert_eq!(classify(true, ""), Access::Key);
        assert_eq!(
            classify(false, "u@h: Permission denied (publickey,password)."),
            Access::NoKey
        );
        assert_eq!(
            classify(
                false,
                "No ED25519 host key is known for h and you have requested strict checking.\r\nHost key verification failed."
            ),
            Access::HostUnknown
        );
        assert_eq!(
            classify(false, "ssh: connect to host h port 22: Connection refused"),
            Access::Unreachable
        );
    }
}
