//! Shared local Codex file/process guards; no graphical or credential dependencies.
use std::{
    fs,
    io::{self, Read},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};
fn invalid(message: &str) -> io::Error {
    io::Error::other(message)
}
fn codex_in_group(pid: u32, group: i32, depth: usize, budget: &mut usize) -> bool {
    if depth > 4 || *budget == 0 {
        return false;
    }
    *budget -= 1;
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
        .any(|child| codex_in_group(child, group, depth + 1, budget))
}

pub fn validate_file(path: &str) -> io::Result<()> {
    // A short literal path is essential: never send shell syntax, newlines or large-paste text.
    if !Path::new(&path).is_absolute()
        || path.len() > 512
        || !path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "attachment requires a short absolute plain PNG path",
        ));
    }
    let parent = Path::new(&path)
        .parent()
        .ok_or_else(|| invalid("staging parent missing"))?;
    let parent_meta =
        fs::symlink_metadata(parent).map_err(|_| invalid("staging parent unavailable"))?;
    if !parent_meta.is_dir()
        || parent_meta.uid() != unsafe { libc::geteuid() }
        || parent_meta.mode() & 0o777 != 0o700
    {
        return Err(invalid("staging parent must be owned and 0700"));
    }
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(&path)
        .map_err(|_| invalid("staged PNG unavailable"))?;
    let m = file
        .metadata()
        .map_err(|_| invalid("staged PNG unavailable"))?;
    if !m.is_file()
        || m.uid() != unsafe { libc::geteuid() }
        || m.mode() & 0o777 != 0o600
        || m.nlink() != 1
        || m.len() > 64 * 1024 * 1024
    {
        return Err(invalid(
            "PNG must be an owned 0600 regular file within the cap",
        ));
    }
    let mut signature = [0; 8];
    (&file)
        .read_exact(&mut signature)
        .map_err(|_| invalid("PNG signature missing"))?;
    if signature != *b"\x89PNG\r\n\x1a\n" {
        return Err(invalid("PNG signature rejected"));
    }

    Ok(())
}
pub fn validate_process(foreground: u32) -> io::Result<()> {
    if foreground == 0 || !codex_in_group(foreground, foreground as i32, 0, &mut 64) {
        return Err(invalid("live same-user Codex foreground required"));
    }
    Ok(())
}
pub fn validate(path: &str, foreground: u32) -> io::Result<()> {
    validate_file(path)?;
    validate_process(foreground)
}
pub fn model_ready(screen: &str) -> bool {
    screen.contains("GPT-") || screen.contains("context left")
}
pub fn highest_image(screen: &str) -> u64 {
    // Only the current composer counts: old turns and model output are not attachment receipts.
    let lines: Vec<_> = screen.lines().collect();
    let Some(start) = lines
        .iter()
        .rposition(|line| line.trim_start().starts_with('›'))
    else {
        return 0;
    };
    let composer = lines[start..]
        .iter()
        .take_while(|line| line.trim_start().starts_with('›') || !model_ready(line))
        .copied()
        .collect::<Vec<_>>()
        .join("\n");
    composer
        .split("[Image #")
        .skip(1)
        .filter_map(|s| s.split_once(']').and_then(|(n, _)| n.parse::<u64>().ok()))
        .max()
        .unwrap_or(0)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_and_model_output_cannot_acknowledge_a_draft() {
        assert_eq!(
            highest_image(
                "› [Image #9] old turn\n• [Image #40] model output\n› New draft\nGPT-6.1-Sol"
            ),
            0
        );
        assert_eq!(
            highest_image("› [Image #9] old turn\n  › [Image #1] new draft\nGPT-6.1-Sol"),
            1
        );
        assert_eq!(highest_image("› [Image #1] Explain GPT-5\nGPT-6.1-Sol"), 1);
        assert_eq!(
            highest_image("› [Image #1] wrapped\n  [Image #2]\nGPT-6.1-Sol"),
            2
        );
    }
    #[test]
    fn process_scan_has_a_global_budget() {
        assert!(!codex_in_group(std::process::id(), 1, 0, &mut 0));
    }
}
