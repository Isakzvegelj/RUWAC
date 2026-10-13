//! Explicit OpenSSH jump aliases, with an anonymous socket rather than a listener.
use std::{
    os::{
        fd::{AsRawFd, OwnedFd, RawFd},
        unix::{net::UnixStream, process::CommandExt},
    },
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

pub fn validate(alias: &str) -> Result<(), String> {
    if alias.is_empty()
        || alias.len() > 128
        || alias.starts_with('-')
        || !alias
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err("Use one configured SSH gateway alias, without arguments or a username".into());
    }
    Ok(())
}

fn gateway_args(alias: &str, host: &str, port: u16) -> Result<Vec<String>, String> {
    validate(alias)?;
    if port == 0
        || host.is_empty()
        || host.starts_with('-')
        || host.len() > 255
        || !host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-:%".contains(&b))
    {
        return Err("Invalid SSH target for gateway forwarding".into());
    }
    let mut args = vec!["-T".into()];
    for option in [
        "BatchMode=yes",
        "StrictHostKeyChecking=yes",
        "UpdateHostKeys=no",
        "ForwardAgent=no",
        "ClearAllForwardings=yes",
        "ControlMaster=no",
        "ControlPath=none",
        "PreferredAuthentications=publickey",
        "PasswordAuthentication=no",
        "KbdInteractiveAuthentication=no",
        "ConnectTimeout=8",
        "ServerAliveInterval=5",
        "ServerAliveCountMax=2",
        "LogLevel=ERROR",
    ] {
        args.extend(["-o".into(), option.into()]);
    }
    args.extend([
        "-W".into(),
        format!("[{host}]:{port}"),
        "--".into(),
        alias.into(),
    ]);
    Ok(args)
}

pub fn proxy_command(alias: &str, host: &str, port: u16) -> Result<String, String> {
    let args = std::iter::once("ssh".to_owned()).chain(gateway_args(alias, host, port)?);
    // OpenSSH expands percent tokens before invoking the proxy shell, including
    // inside quotes. Escape that layer first, then quote every literal argument.
    Ok(args
        .map(|s| format!("'{}'", s.replace('%', "%%").replace('\'', "'\\''")))
        .collect::<Vec<_>>()
        .join(" "))
}

pub struct GatewayStream {
    stream: UnixStream,
    child: Child,
}
impl AsRawFd for GatewayStream {
    fn as_raw_fd(&self) -> RawFd {
        self.stream.as_raw_fd()
    }
}
impl Drop for GatewayStream {
    fn drop(&mut self) {
        let _ = self.stream.shutdown(std::net::Shutdown::Both);
        if self.child.try_wait().is_ok_and(|s| s.is_none()) {
            // This group was created solely for this transport. It cannot be an
            // existing multiplexed master or another owner's SSH connection.
            let group = self.child.id() as i32;
            unsafe {
                libc::kill(-group, libc::SIGTERM);
            }
            let deadline = Instant::now() + Duration::from_millis(250);
            while Instant::now() < deadline {
                if self.child.try_wait().is_ok_and(|s| s.is_some()) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            unsafe {
                libc::kill(-group, libc::SIGKILL);
            }
            let _ = self.child.wait();
        }
    }
}

pub fn open(alias: &str, host: &str, port: u16) -> Result<GatewayStream, String> {
    let args = gateway_args(alias, host, port)?;
    let (stream, peer) = UnixStream::pair().map_err(|_| "SSH gateway socket unavailable")?;
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .map_err(|_| "SSH gateway timeout unavailable")?;
    stream
        .set_write_timeout(Some(Duration::from_secs(15)))
        .map_err(|_| "SSH gateway timeout unavailable")?;
    let input: OwnedFd = peer
        .try_clone()
        .map_err(|_| "SSH gateway socket unavailable")?
        .into();
    let output: OwnedFd = peer.into();
    let child = Command::new("ssh")
        .args(args)
        .stdin(Stdio::from(input))
        .stdout(Stdio::from(output))
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|_| "OpenSSH gateway process unavailable")?;
    Ok(GatewayStream { stream, child })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_alias_and_target_validation_precedes_process_start() {
        for alias in [
            "",
            "-option",
            "user@host",
            "jump;command",
            "jump alias",
            "jump\n",
        ] {
            assert!(validate(alias).is_err());
        }
        assert!(validate("generated-gateway_1").is_ok());
        assert!(gateway_args("generated", "target;command", 22).is_err());
        assert!(gateway_args("generated", "target", 0).is_err());
    }
    #[test]
    fn gateway_authentication_is_separate_and_never_reuses_target_credentials() {
        let args = gateway_args("generated-gateway", "2001:db8::1", 2222).unwrap();
        assert!(args.contains(&"StrictHostKeyChecking=yes".into()));
        assert!(args.contains(&"PreferredAuthentications=publickey".into()));
        assert!(args.contains(&"ForwardAgent=no".into()));
        assert!(args.contains(&"ControlPath=none".into()));
        assert!(!args.contains(&"-i".into()));
        assert_eq!(args.last().unwrap(), "generated-gateway");
    }
    #[test]
    fn proxy_percent_expansion_is_escaped_before_literal_quoting() {
        let proxy = proxy_command("generated", "fe80::1%interface", 22).unwrap();
        assert!(proxy.contains("'[fe80::1%%interface]:22'"));
        assert!(proxy.ends_with("'--' 'generated'"));
    }
}
