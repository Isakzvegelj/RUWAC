//! Local SSH identities the current user already has. Only public-key files
//! and ~/.ssh/config are read; private key contents are never opened.
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq)]
pub struct Key {
    /// Private key path, as saved in a profile's identity file.
    pub path: PathBuf,
    /// Key type from the public key, e.g. "ssh-ed25519".
    pub kind: String,
    pub comment: String,
}
impl Key {
    pub fn label(&self) -> String {
        let name = self
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let kind = self.kind.trim_start_matches("ssh-");
        if self.comment.is_empty() {
            format!("{name} · {kind}")
        } else {
            format!("{name} · {kind} · {}", self.comment)
        }
    }
}

/// Preferred first: modern key types, then the conventional default names.
fn rank(key: &Key) -> (u8, u8, String) {
    let kind = match key.kind.as_str() {
        k if k.contains("ed25519") => 0,
        k if k.contains("ecdsa") => 1,
        "ssh-rsa" => 2,
        _ => 3,
    };
    let name = key
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let default_name = u8::from(!name.starts_with("id_"));
    (kind, default_name, name)
}

fn public_key(private: &Path) -> Option<Key> {
    let mut public = private.as_os_str().to_owned();
    public.push(".pub");
    let meta = fs::symlink_metadata(&public).ok()?;
    if !meta.is_file() || meta.len() > 16 * 1024 || !private.is_file() {
        return None;
    }
    let bytes = crate::credentials::bounded_file(Path::new(&public), 16 * 1024).ok()?;
    let text = std::str::from_utf8(&bytes).ok()?;
    let mut parts = text.split_whitespace();
    let kind = parts.next()?.to_owned();
    if !(kind.starts_with("ssh-") || kind.starts_with("ecdsa-") || kind.starts_with("sk-")) {
        return None;
    }
    parts.next()?;
    Some(Key {
        path: private.to_path_buf(),
        kind,
        comment: parts.collect::<Vec<_>>().join(" "),
    })
}

/// IdentityFile entries from an ssh_config, with ~ expanded.
fn config_identities(config: &str, home: &Path) -> Vec<PathBuf> {
    config
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let (key, value) = line.split_once(|c: char| c.is_whitespace() || c == '=')?;
            if !key.eq_ignore_ascii_case("IdentityFile") {
                return None;
            }
            let value = value.trim_start_matches(|c: char| c.is_whitespace() || c == '=');
            let value = value.trim().trim_matches('"');
            if value.is_empty() || value.contains('%') {
                return None;
            }
            Some(match value.strip_prefix("~/") {
                Some(rest) => home.join(rest),
                None => PathBuf::from(value),
            })
        })
        .collect()
}

pub fn detect_in(home: &Path) -> Vec<Key> {
    let ssh = home.join(".ssh");
    let mut candidates: Vec<PathBuf> = fs::read_dir(&ssh)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_none_or(|e| e != "pub"))
        .take(4096)
        .collect();
    if let Ok(bytes) = crate::credentials::bounded_file(&ssh.join("config"), 1024 * 1024) {
        if let Ok(config) = std::str::from_utf8(&bytes) {
            candidates.extend(config_identities(config, home).into_iter().take(4096));
        }
    }
    let mut keys: Vec<Key> = candidates.iter().filter_map(|p| public_key(p)).collect();
    keys.sort_by_key(rank);
    keys.dedup_by(|a, b| a.path == b.path);
    keys
}

pub fn detect() -> Vec<Key> {
    detect_in(&gtk::glib::home_dir())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_keys_with_public_halves_and_config_identities_in_preference_order() {
        let home = std::env::temp_dir().join(ruwac_protocol::id("ssh-keys-test"));
        let ssh = home.join(".ssh");
        let other = home.join("keys");
        fs::create_dir_all(&ssh).unwrap();
        fs::create_dir_all(&other).unwrap();
        for (path, public) in [
            (ssh.join("id_rsa"), "ssh-rsa AAAA user@rsa"),
            (ssh.join("id_ed25519"), "ssh-ed25519 AAAA user@laptop"),
            (other.join("work"), "ssh-ed25519 AAAA"),
        ] {
            fs::write(&path, "private").unwrap();
            fs::write(format!("{}.pub", path.display()), public).unwrap();
        }
        // No public half, unrelated files and percent tokens are ignored.
        fs::write(ssh.join("lonely"), "private").unwrap();
        fs::write(ssh.join("known_hosts"), "host ssh-ed25519 AAAA").unwrap();
        fs::write(
            ssh.join("config"),
            "Host work\n  IdentityFile ~/keys/work\n  IdentityFile ~/.ssh/%h\nHost *\n IdentityFile=~/.ssh/id_rsa\n",
        )
        .unwrap();
        let keys = detect_in(&home);
        let names: Vec<_> = keys.iter().map(|k| k.label()).collect();
        assert_eq!(
            names,
            [
                "id_ed25519 · ed25519 · user@laptop",
                "work · ed25519",
                "id_rsa · rsa · user@rsa"
            ]
        );
        assert_eq!(keys[1].path, other.join("work"));
        fs::remove_dir_all(home).unwrap();
        assert!(detect_in(Path::new("/nonexistent-ruwac-home")).is_empty());
    }
}
