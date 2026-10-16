//! Ctrl+V with an image in an SSH tab: a program on the remote host (Codex,
//! Claude) cannot read this computer's clipboard, so the PNG is copied over
//! the tab's own SSH endpoint into a private remote folder and its remote
//! path is pasted, which those agents attach as an image.
use crate::{profiles::Profile, view::View};
use gtk::{gdk, glib, prelude::*};
use std::{
    cell::RefCell,
    collections::HashMap,
    io::{Read, Write},
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};

thread_local! {
    static TARGETS: RefCell<HashMap<String, Profile>> = RefCell::new(HashMap::new());
}

/// Record the SSH endpoint a terminal session was opened for.
pub fn remember(session: &str, profile: &Profile) {
    if profile.protocol == "SSH" {
        TARGETS.with(|t| t.borrow_mut().insert(session.into(), profile.clone()));
    }
}

fn image_only(clipboard: &gdk::Clipboard) -> bool {
    let formats = clipboard.formats();
    formats.contain_mime_type("image/png")
        && !formats.contain_mime_type("text/plain")
        && !formats.contain_mime_type("text/plain;charset=utf-8")
}

/// True when the paste was taken over (SSH tab, image-only clipboard).
pub fn try_paste(view: &Rc<View>) -> bool {
    let Some(profile) = TARGETS.with(|t| t.borrow().get(&view.session).cloned()) else {
        return false;
    };
    let clipboard = view.area.display().clipboard();
    if !image_only(&clipboard) {
        return false;
    }
    view.status
        .set_text(&format!("Copying clipboard image to {}…", profile.name));
    let weak = Rc::downgrade(view);
    clipboard.read_texture_async(None::<&gtk::gio::Cancellable>, move |result| {
        let Some(view) = weak.upgrade() else {
            return;
        };
        let png = match result {
            Ok(Some(texture)) => texture.save_to_png_bytes().to_vec(),
            _ => {
                view.warn("Clipboard image unavailable");
                return;
            }
        };
        if png.len() > ruwac_clipboard::MAX_IMAGE_BYTES {
            view.warn("Clipboard image is too large to send");
            return;
        }
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let _ = tx.send(upload(&profile, &png));
        });
        let weak = Rc::downgrade(&view);
        glib::timeout_add_local(Duration::from_millis(50), move || {
            let Some(view) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            match rx.try_recv() {
                Ok(Ok(path)) => {
                    view.paste(path.clone());
                    view.status.set_text(&format!("Image sent: {path}"));
                    glib::ControlFlow::Break
                }
                Ok(Err(e)) => {
                    view.warn(&format!("Image not sent: {e}"));
                    glib::ControlFlow::Break
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(_) => glib::ControlFlow::Break,
            }
        });
    });
    true
}

/// Remote command: private folder, unique name, absolute path on stdout.
fn remote_command(name: &str) -> String {
    format!(
        "umask 077; d=\"$HOME/.cache/ruwac/images\"; mkdir -p \"$d\" && f=\"$d/{name}.png\" && cat > \"$f\" && printf %s \"$f\""
    )
}

fn upload(profile: &Profile, png: &[u8]) -> Result<String, String> {
    let mut argv = crate::profiles::ssh_argv(profile)?;
    let host = argv
        .iter()
        .position(|a| a == "--")
        .ok_or("Invalid SSH command")?;
    argv.splice(
        host..host,
        [
            "-T",
            "-o",
            "ConnectTimeout=10",
            "-o",
            "ForwardAgent=no",
            "-o",
            "ClearAllForwardings=yes",
        ]
        .map(String::from),
    );
    argv.push(remote_command(&ruwac_protocol::id("image")));
    let mut command = Command::new(&argv[0]);
    command
        .args(&argv[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if profile.preferences.ssh_auth == "password" {
        if let Ok(exe) = std::env::current_exe() {
            command
                .env("SSH_ASKPASS", exe)
                .env("SSH_ASKPASS_REQUIRE", "force")
                .env(crate::askpass::REFERENCE, &profile.secret_reference);
        }
    }
    let mut child = command.spawn().map_err(|_| "ssh unavailable".to_string())?;
    let written = child.stdin.take().map(|mut s| s.write_all(png));
    let deadline = Instant::now() + Duration::from_secs(60);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("timed out".into());
            }
        }
    };
    let (mut out, mut err) = (String::new(), String::new());
    if let Some(mut s) = child.stdout.take() {
        let _ = s.by_ref().take(4096).read_to_string(&mut out);
    }
    if let Some(mut s) = child.stderr.take() {
        let _ = s.by_ref().take(4096).read_to_string(&mut err);
    }
    if !status.success() || !matches!(written, Some(Ok(()))) || !out.starts_with('/') {
        let reason = err
            .lines()
            .last()
            .unwrap_or("upload failed")
            .trim()
            .to_string();
        return Err(if reason.is_empty() {
            "upload failed".into()
        } else {
            reason
        });
    }
    Ok(out.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remote_command_writes_private_file_and_prints_its_path() {
        let c = remote_command("image_abc");
        assert!(c.starts_with("umask 077;"));
        assert!(c.contains("$HOME/.cache/ruwac/images"));
        assert!(c.ends_with("printf %s \"$f\""));
        assert!(c.contains("image_abc.png"));
    }
}
