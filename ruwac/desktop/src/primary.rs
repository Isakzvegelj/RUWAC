//! Optional authenticated Linux PRIMARY bridge. CLIPBOARD remains independent.
use crate::profiles::{self, Profile};
use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Child, ChildStdin, Command, Stdio},
    time::{Duration, Instant},
};
const HELPER: &str = include_str!("../../../scripts/primary-selection.py");
fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
pub fn validate(p: &profiles::Preferences) -> Result<(), String> {
    if !p.primary_sync {
        return Ok(());
    }
    if p.primary_user.is_empty()
        || p.primary_user.len() > 64
        || !p
            .primary_user
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
        || p.primary_user.starts_with('-')
        || !p
            .primary_display
            .strip_prefix("wayland-")
            .is_some_and(|s| !s.is_empty() && s.len() <= 3 && s.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err("Mouse selection sharing needs a Linux SSH account and exact Wayland session (for example wayland-0)".into());
    }
    Ok(())
}
pub fn ssh_argv(p: &Profile) -> Result<Vec<String>, String> {
    validate(&p.preferences)?;
    if p.protocol != "RDP" || !p.preferences.primary_sync {
        return Err("Mouse selection sharing is disabled".into());
    }
    if !p.preferences.gateway.is_empty() && p.preferences.ssh_jump_alias.is_empty() {
        return Err("Selection sharing needs an explicit SSH route".into());
    }
    let (host, _) = profiles::endpoint(&p.server, 3389)?;
    let mut ssh = p.clone();
    ssh.protocol = "SSH".into();
    ssh.username = p.preferences.primary_user.clone();
    ssh.server = if p.preferences.primary_port == 0 {
        host
    } else if host.contains(':') {
        format!("[{host}]:{}", p.preferences.primary_port)
    } else {
        format!("{host}:{}", p.preferences.primary_port)
    };
    let mut args = profiles::ssh_argv(&ssh)?;
    for v in &mut args {
        if v == "StrictHostKeyChecking=ask" {
            *v = "StrictHostKeyChecking=yes".into();
        }
    }
    let i = args
        .iter()
        .position(|s| s == "--")
        .ok_or("Invalid SSH route")?;
    args.splice(
        i..i,
        [
            "-T".into(),
            "-o".into(),
            "BatchMode=yes".into(),
            "-o".into(),
            "ForwardAgent=no".into(),
            "-o".into(),
            "ClearAllForwardings=yes".into(),
            "-o".into(),
            "ConnectTimeout=8".into(),
            "-o".into(),
            "ServerAliveInterval=3".into(),
            "-o".into(),
            "ServerAliveCountMax=2".into(),
        ],
    );
    args.push(format!(
        "python3 -c {} --peer {}",
        quote(HELPER),
        quote(&p.preferences.primary_display)
    ));
    Ok(args)
}
#[derive(Default)]
struct Incoming {
    ready: bool,
    failed: bool,
    text: Option<(u64, String)>,
}
pub struct Bridge {
    child: Option<Child>,
    input: Option<ChildStdin>,
    incoming: std::sync::Arc<std::sync::Mutex<Incoming>>,
    last: Instant,
    modes: Option<bool>,
    epoch: u64,
    pub ready: bool,
    pub failed: bool,
}
impl Bridge {
    pub fn start(profile: &Profile) -> Result<Self, String> {
        let args = ssh_argv(profile)?;
        let mut child = Command::new(&args[0])
            .args(&args[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "SSH is unavailable for mouse selection sharing")?;
        let input = child.stdin.take();
        let output = child.stdout.take().unwrap();
        let incoming = std::sync::Arc::new(std::sync::Mutex::new(Incoming::default()));
        let state = incoming.clone();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(output);
            loop {
                let mut line = Vec::new();
                let Ok(n) = (&mut reader)
                    .take((1024 * 1024 * 6 + 1025) as u64)
                    .read_until(b'\n', &mut line)
                else {
                    break;
                };
                if n == 0 || n > 1024 * 1024 * 6 + 1024 || line.last() != Some(&b'\n') {
                    break;
                }
                let Ok(value) = serde_json::from_slice::<serde_json::Value>(&line) else {
                    break;
                };
                match value["type"].as_str() {
                    Some("hello") => state.lock().unwrap().ready = true,
                    Some("text") => {
                        let (Some(epoch), Some(text)) =
                            (value["epoch"].as_u64(), value["text"].as_str())
                        else {
                            break;
                        };
                        if text.is_empty() || text.len() > 1024 * 1024 || text.contains('\0') {
                            break;
                        }
                        state.lock().unwrap().text = Some((epoch, text.to_owned()));
                    }
                    _ => break,
                }
            }
            state.lock().unwrap().failed = true;
        });
        Ok(Self {
            child: Some(child),
            input,
            incoming,
            last: Instant::now() - Duration::from_secs(2),
            modes: None,
            epoch: 0,
            ready: false,
            failed: false,
        })
    }
    pub fn tick(&mut self, _send: bool, receive: bool) {
        {
            let state = self.incoming.lock().unwrap();
            self.ready = state.ready && !state.failed;
            self.failed = state.failed;
        }
        if self
            .child
            .as_mut()
            .is_some_and(|c| c.try_wait().ok().flatten().is_some())
        {
            self.failed = true;
            self.ready = false;
        }
        if self.failed {
            self.input.take();
            return;
        }
        let changed = self.modes != Some(receive);
        if !changed && self.last.elapsed() < Duration::from_secs(1) {
            return;
        }
        if changed {
            self.epoch = self.epoch.saturating_add(1);
            self.incoming.lock().unwrap().text.take();
        }
        self.modes = Some(receive);
        self.last = Instant::now();
        if let Some(input) = self.input.as_mut() {
            if writeln!(
                input,
                "{{\"type\":\"mode\",\"epoch\":{},\"send\":{receive},\"receive\":false}}",
                self.epoch
            )
            .is_err()
            {
                self.failed = true;
                self.ready = false;
                self.input.take();
            }
        }
    }
    pub fn publish_primary(
        &self,
        clipboard: &gtk::gdk::Clipboard,
    ) -> Result<Option<usize>, String> {
        let Some(text) = self.take_text() else {
            return Ok(None);
        };
        let size = text.len();
        let provider = gtk::gdk::ContentProvider::for_bytes(
            "text/plain;charset=utf-8",
            &gtk::glib::Bytes::from_owned(text.into_bytes()),
        );
        clipboard
            .set_content(Some(&provider))
            .map_err(|_| "The desktop refused the remote mouse selection".to_string())?;
        Ok(Some(size))
    }
    pub fn take_text(&self) -> Option<String> {
        self.incoming
            .lock()
            .unwrap()
            .text
            .take()
            .filter(|(epoch, _)| self.modes == Some(true) && *epoch == self.epoch)
            .map(|(_, text)| text)
    }
}
impl Drop for Bridge {
    fn drop(&mut self) {
        self.input.take();
        if let Some(mut child) = self.child.take() {
            std::thread::spawn(move || {
                let end = Instant::now() + Duration::from_secs(5);
                loop {
                    if child.try_wait().ok().flatten().is_some() {
                        break;
                    }
                    if Instant::now() >= end {
                        let _ = child.kill();
                        let _ = child.wait();
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(25));
                }
            });
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_route_and_session() {
        let mut p:Profile=serde_json::from_value(serde_json::json!({"name":"test","group":"","protocol":"RDP","server":"server:3389","username":"service","secret_reference":"x","favorite":false,"last_used":0,"tags":[]})).unwrap();
        p.preferences.primary_sync = true;
        p.preferences.primary_user = "desktop".into();
        p.preferences.primary_display = "wayland-0".into();
        p.preferences.primary_port = 9223;
        let a = ssh_argv(&p).unwrap();
        assert!(a.windows(2).any(|v| v == ["-l", "desktop"]));
        assert!(a.windows(2).any(|v| v == ["-p", "9223"]));
        assert!(a.iter().any(|s| s == "StrictHostKeyChecking=yes"));
        assert!(
            !a.iter()
                .take(a.len() - 1)
                .any(|s| s.contains("ask") || s == "service")
        );
        for display in ["", "wayland-0;evil", "/tmp/wayland-0", "wayland-1000"] {
            p.preferences.primary_display = display.into();
            assert!(ssh_argv(&p).is_err());
        }
    }
}
