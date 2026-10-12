//! Explicit Codex draft attachment. Never submits a turn or falls back to shell input.
use crate::{Error, Result, State};
use serde_json::{Value, json};
use std::{
    fs,
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};

fn codex_in_group(pid: u32, group: i32, depth: usize) -> bool {
    if depth > 4 {
        return false;
    }
    let root = format!("/proc/{pid}");
    if fs::metadata(&root).map_or(true, |m| m.uid() != unsafe { libc::geteuid() }) {
        return false;
    }
    let Ok(stat) = fs::read_to_string(format!("{root}/stat")) else {
        return false;
    };
    let Some((_, tail)) = stat.rsplit_once(')') else {
        return false;
    };
    let fields: Vec<_> = tail.split_whitespace().collect();
    if fields.get(2).and_then(|v| v.parse::<i32>().ok()) != Some(group) {
        return false;
    }
    if fs::read_link(format!("{root}/exe"))
        .ok()
        .and_then(|p| p.file_name().map(|n| n == "codex"))
        == Some(true)
    {
        return true;
    }
    fs::read_to_string(format!("{root}/task/{pid}/children"))
        .unwrap_or_default()
        .split_whitespace()
        .take(32)
        .filter_map(|s| s.parse().ok())
        .any(|child| codex_in_group(child, group, depth + 1))
}

impl State {
    pub(super) fn attach_image(&self, v: &Value) -> Result<Value> {
        if v.as_object().is_none_or(|o| o.len() != 3) {
            return Err(Error::new(
                "INVALID_PARAMS",
                "pane, path and foreground_pid required",
            ));
        }
        let p = self.pane(v)?;
        let path = crate::text(v, "path")?;
        // A short literal path is essential: never send shell syntax, newlines or large-paste text.
        if !Path::new(&path).is_absolute()
            || path.len() > 512
            || !path
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b))
        {
            return Err(Error::new(
                "INVALID_PATH",
                "attachment requires a short absolute plain PNG path",
            ));
        }
        let parent = Path::new(&path)
            .parent()
            .ok_or_else(|| Error::new("INVALID_PATH", "staging parent missing"))?;
        let parent_meta = fs::symlink_metadata(parent)
            .map_err(|_| Error::new("INVALID_IMAGE", "staging parent unavailable"))?;
        if !parent_meta.is_dir()
            || parent_meta.uid() != unsafe { libc::geteuid() }
            || parent_meta.mode() & 0o777 != 0o700
        {
            return Err(Error::new(
                "INVALID_IMAGE",
                "staging parent must be owned and 0700",
            ));
        }
        let file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&path)
            .map_err(|_| Error::new("INVALID_IMAGE", "staged PNG unavailable"))?;
        let m = file
            .metadata()
            .map_err(|_| Error::new("INVALID_IMAGE", "staged PNG unavailable"))?;
        if !m.is_file()
            || m.uid() != unsafe { libc::geteuid() }
            || m.mode() & 0o777 != 0o600
            || m.nlink() != 1
            || m.len() > 64 * 1024 * 1024
        {
            return Err(Error::new(
                "INVALID_IMAGE",
                "PNG must be an owned 0600 regular file within the cap",
            ));
        }
        let mut signature = [0; 8];
        (&file)
            .read_exact(&mut signature)
            .map_err(|_| Error::new("INVALID_IMAGE", "PNG signature missing"))?;
        if signature != *b"\x89PNG\r\n\x1a\n" {
            return Err(Error::new("INVALID_IMAGE", "PNG signature rejected"));
        }
        let w = &self.windows[&p.window];
        let s = &self.sessions[&w.session];
        let pty = p.live_pty()?;
        let group = pty
            .foreground_group()
            .filter(|g| *g > 0)
            .ok_or_else(|| Error::new("STALE_TARGET", "foreground unavailable"))?;
        let screen = p.term.capture(100);
        // Codex can paint its empty composer before model/image capabilities are loaded.
        // Refuse that startup state rather than inserting a literal filename into the draft.
        let model_ready = screen.contains("GPT-") || screen.contains("context left");
        if v["foreground_pid"].as_i64() != Some(group as i64)
            || w.active != p.id
            || s.active != w.id
            || !model_ready
            || !p.term.paste()
            || !codex_in_group(group as u32, group, 0)
        {
            return Err(Error::new(
                "STALE_TARGET",
                "selected foreground Codex with bracketed paste required",
            ));
        }
        if pty.foreground_group() != Some(group) {
            return Err(Error::new("STALE_TARGET", "foreground changed"));
        }
        pty.send(format!("\x1b[200~{path}\x1b[201~").into_bytes())
            .map_err(|e| Error::new("BACKPRESSURE", e))?;
        Ok(
            json!({"pane":p.id,"queued":true,"attached":false,"submitted":false,"adapter":"codex","foreground_pid":group}),
        )
    }
}
