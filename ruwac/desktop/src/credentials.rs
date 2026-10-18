//! System Secret Service adapter. Secrets travel through a pipe, never argv or metadata.
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
fn valid(reference: &str) -> Result<(), String> {
    if reference.is_empty() || reference.len() > 1024 || reference.chars().any(char::is_control) {
        Err("Invalid credential reference".into())
    } else {
        Ok(())
    }
}
fn helper(args: &[&str], secret: Option<String>) -> Result<Vec<u8>, String> {
    let mut child = Command::new("secret-tool")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Secret Service helper unavailable".to_string())?;
    if let Some(mut value) = secret {
        if value.is_empty() || value.len() > 4096 || value.contains('\0') {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Credential must contain 1–4096 UTF-8 bytes".into());
        }
        let result = child.stdin.take().unwrap().write_all(value.as_bytes());
        unsafe { value.as_bytes_mut().fill(0) };
        if result.is_err() {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Secret Service pipe unavailable".into());
        }
    }
    drop(child.stdin.take());
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut b = Vec::new();
        let r = stdout.take(4097).read_to_end(&mut b);
        let _ = tx.send((r, b));
    });
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let (r, b) = rx
                    .recv_timeout(Duration::from_secs(1))
                    .map_err(|_| "Secret Service output unavailable")?;
                if !status.success() || r.is_err() || b.len() > 4096 {
                    return Err("Secret Service unavailable, locked, or reference missing".into());
                }
                return Ok(b);
            }
            Ok(None) if started.elapsed() < Duration::from_secs(8) => {
                std::thread::sleep(Duration::from_millis(20))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Secret Service timed out; unlock the keyring and retry".into());
            }
        }
    }
}
pub fn lookup(reference: &str) -> Result<String, String> {
    valid(reference)?;
    let b = helper(
        &["lookup", "application", "RUWAC", "profile", reference],
        None,
    )?;
    if b.is_empty() {
        return Err("Credential reference missing or locked".into());
    }
    String::from_utf8(b)
        .map(|s| s.trim_end_matches('\n').to_owned())
        .map_err(|_| "Credential is not UTF-8".into())
}
/// Read the selected Remmina profile's Secret Service item. Never interpret
/// an encrypted value from a .remmina file as a plaintext password.
pub fn lookup_remmina(path: &std::path::Path) -> Result<String, String> {
    let bytes = bounded_file(path, 65536)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| "Invalid Remmina text")?;
    let (mut section, mut keyring) = (false, false);
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            section = line == "[remmina]";
        } else if section {
            if let Some((key, value)) = line.split_once('=') {
                if key.trim() == "password" {
                    keyring = value.trim() == ".";
                }
            }
        }
    }
    if !keyring {
        return Err("Remmina password is not saved in Secret Service; set it in Edit".into());
    }
    let path = path
        .canonicalize()
        .map_err(|_| "Remmina file unavailable")?;
    let filename = path.to_str().ok_or("Invalid Remmina filename")?;
    valid(filename)?;
    let b = helper(
        &[
            "lookup",
            "xdg:schema",
            "org.remmina.Password",
            "filename",
            filename,
            "key",
            "password",
        ],
        None,
    )?;
    if b.is_empty() {
        return Err("Remmina password missing or keyring locked".into());
    }
    String::from_utf8(b)
        .map(|s| s.trim_end_matches('\n').to_owned())
        .map_err(|_| "Remmina password is not UTF-8".into())
}
pub fn set(reference: &str, secret: String) -> Result<(), String> {
    valid(reference)?;
    helper(
        &[
            "store",
            "--label=RUWAC endpoint",
            "application",
            "RUWAC",
            "profile",
            reference,
        ],
        Some(secret),
    )
    .map(|_| ())
}
pub fn forget(reference: &str) -> Result<(), String> {
    valid(reference)?;
    helper(
        &["clear", "application", "RUWAC", "profile", reference],
        None,
    )
    .map(|_| ())
}

/// Explicit trust/key selections must not turn a special file into an unbounded read.
pub fn bounded_file(path: &std::path::Path, limit: u64) -> Result<Vec<u8>, String> {
    use std::os::unix::fs::OpenOptionsExt;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| "Select a readable regular file, without a symlink")?;
    if !file
        .metadata()
        .is_ok_and(|m| m.is_file() && m.len() <= limit)
    {
        return Err("Trust/key file must be regular and within the size cap".into());
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read trust/key file")?;
    if bytes.len() as u64 > limit {
        return Err("Trust/key file grew beyond the size cap".into());
    }
    Ok(bytes)
}
#[cfg(test)]
mod file_tests {
    use super::*;
    #[test]
    fn bounded_trust_key_reads_reject_special_symlink_and_oversize() {
        use std::os::unix::{
            ffi::OsStrExt,
            fs::{DirBuilderExt, symlink},
        };
        let root = std::env::temp_dir().join(ruwac_protocol::id("trust-file"));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&root)
            .unwrap();
        let path = root.join("generated");
        std::fs::write(&path, b"fixture").unwrap();
        assert_eq!(bounded_file(&path, 8).unwrap().len(), 7);
        assert!(bounded_file(&path, 2).is_err());
        let link = root.join("link");
        symlink(&path, &link).unwrap();
        assert!(bounded_file(&link, 8).is_err());
        let fifo = root.join("fifo");
        let name = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        assert!(bounded_file(&fifo, 8).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
