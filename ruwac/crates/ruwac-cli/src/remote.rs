//! Authenticated OpenSSH transport. Request data never enters the remote shell command.
use ruwac_protocol::{MAX_FRAME, Request};
use serde_json::Value;
use std::{
    io::{self, Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc,
    time::Duration,
};

pub struct Remote {
    pub host: String,
    pub binary: String,
    pub socket: Option<PathBuf>,
}
fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
impl Remote {
    pub fn validate(&self) -> io::Result<()> {
        if self.host.is_empty()
            || self.host.len() > 255
            || self.host.starts_with('-')
            || !self
                .host
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-@[]:".contains(&b))
            || self.binary.is_empty()
            || self.binary.starts_with('-')
            || self.binary.chars().any(char::is_control)
            || self.socket.as_ref().is_some_and(|p| {
                p.to_str()
                    .is_none_or(|s| s.is_empty() || s.chars().any(char::is_control))
            })
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid SSH destination, binary or socket",
            ));
        }
        Ok(())
    }
    fn command(&self, tty: bool, args: &[&str]) -> Command {
        let mut words = vec![self.binary.clone()];
        if let Some(socket) = &self.socket {
            words.extend(["--socket".into(), socket.to_string_lossy().into()]);
        }
        words.extend(args.iter().map(|s| s.to_string()));
        let mut c = Command::new("ssh");
        c.args([
            if tty { "-tt" } else { "-T" },
            "-o",
            "BatchMode=yes",
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
            "ConnectTimeout=10",
            "-o",
            "ServerAliveInterval=15",
            "-o",
            "ServerAliveCountMax=2",
            "-o",
            "ForwardAgent=no",
            "-o",
            "ClearAllForwardings=yes",
            "--",
        ])
        .arg(&self.host)
        .arg(words.iter().map(|s| quote(s)).collect::<Vec<_>>().join(" "));
        c
    }
    fn exchange(&self, command: &[&str], input: Vec<u8>) -> io::Result<Value> {
        if input.len() > MAX_FRAME {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "request exceeds 1 MiB",
            ));
        }
        let mut child = self
            .command(false, command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::sync_channel(1);
        // Reading and writing run together so a rejecting peer cannot deadlock a large request.
        let writer = std::thread::spawn(move || stdin.write_all(&input));
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let result = stdout
                .take((MAX_FRAME + 1) as u64)
                .read_to_end(&mut bytes)
                .map(|_| bytes);
            let _ = tx.send(result);
        });
        let output = match rx.recv_timeout(Duration::from_secs(70)) {
            Ok(v) => v,
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "SSH response exceeded 70 seconds",
                ));
            }
        };
        let bytes = match output {
            Ok(v) => v,
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e);
            }
        };
        if bytes.len() > MAX_FRAME {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::other("SSH response exceeds 1 MiB"));
        }
        // EOF need not mean process termination: keep the same overall deadline bounded.
        let end = std::time::Instant::now() + Duration::from_secs(2);
        let status = loop {
            if let Some(s) = child.try_wait()? {
                break s;
            }
            if std::time::Instant::now() >= end {
                let _ = child.kill();
                let _ = child.wait();
                return Err(io::Error::other("SSH peer did not exit after response"));
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        let write = writer
            .join()
            .map_err(|_| io::Error::other("SSH input worker failed"))?;
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|_| io::Error::other("SSH peer returned no valid RUWAC JSON response"))?;
        if !status.success() && value.get("error").is_none() {
            return Err(io::Error::other("SSH command failed"));
        }
        if value.get("error").is_none() {
            write?;
        }
        Ok(value)
    }
    pub fn rpc(&self, request: &Request) -> io::Result<Value> {
        let value = self.exchange(&["request"], serde_json::to_vec(request)?)?;
        if value["version"] != 0 || value["id"] != request.id {
            return Err(io::Error::other("remote protocol/correlation rejected"));
        }
        if value.get("error").is_some() {
            println!("{value}");
            return Err(io::Error::other("remote request failed"));
        }
        Ok(value)
    }
    pub fn start(&self) -> io::Result<Value> {
        let v = self.exchange(&["start"], Vec::new())?;
        if v["version"] != 0 || v["result"]["ready"] != true {
            return Err(io::Error::other("remote startup response rejected"));
        }
        Ok(v)
    }
    pub fn attach(&self, session: &str) -> io::Result<()> {
        if !ruwac_protocol::valid_id(session, "session") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid session ID",
            ));
        }
        let status = self
            .command(true, &["attach", "--session", session])
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other("remote terminal attachment failed"))
        }
    }
}
