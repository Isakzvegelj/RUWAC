//! Verified atomic PNG publication over the profile's existing OpenSSH route.
#[path = "bounded_process.rs"]
mod bounded_process;
use crate::profiles::Profile;
use sha2::{Digest, Sha256};
use std::{process::Command, sync::atomic::AtomicBool, time::Duration};

pub fn shutdown() {
    bounded_process::shutdown();
}
fn remote_command(name: &str, bytes: usize, sha256: &str) -> String {
    // All interpolated values are locally generated ASCII IDs/numbers/digests.
    // Existing parent metadata is inspected, never chmodded; only owned new
    // files/directories are created. ln publishes without replacing any file.
    format!(
        r#"umask 077; set -e
for d in "$HOME/.cache" "$HOME/.cache/ruwac" "$HOME/.cache/ruwac/images"; do
 test ! -L "$d"
 if test ! -e "$d"; then mkdir -m 700 -- "$d"; fi
 test -d "$d"
 test -O "$d"
done
test "$(stat -c %a -- "$d")" = 700
t=$(mktemp "$d/.upload-XXXXXXXXXXXX")
trap 'rm -f -- "$t"' EXIT HUP INT TERM
cat > "$t"
test "$(wc -c < "$t")" -eq {bytes}
printf '%s  %s\n' '{sha256}' "$t" | sha256sum --status -c -
f="$d/{name}.png"
ln -- "$t" "$f"
printf '%s\n' "$f"
"#
    )
}
pub fn upload(
    profile: &Profile,
    png: &[u8],
    allowed: &AtomicBool,
    timeout: Duration,
) -> Result<String, String> {
    profile.validate()?;
    if profile.protocol != "SSH" || png.is_empty() || png.len() > ruwac_clipboard::MAX_IMAGE_BYTES {
        return Err("Invalid SSH image upload".into());
    }
    let mut argv = crate::profiles::ssh_argv(profile)?;
    let at = argv
        .iter()
        .position(|a| a == "--")
        .ok_or("Invalid SSH command")?;
    argv.splice(
        at..at,
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
    for argument in &mut argv {
        if argument == "StrictHostKeyChecking=ask" {
            *argument = "StrictHostKeyChecking=yes".into();
        }
    }
    if profile.preferences.ssh_auth != "password"
        && profile.preferences.identity_secret_reference.is_empty()
    {
        argv.splice(at..at, ["-o".into(), "BatchMode=yes".into()]);
    }
    let name = ruwac_protocol::id("image");
    argv.splice(at..at, ["-o".into(), "NumberOfPasswordPrompts=1".into()]);
    argv.push(remote_command(
        &name,
        png.len(),
        &format!("{:x}", Sha256::digest(png)),
    ));
    let mut command = Command::new(&argv[0]);
    command.args(&argv[1..]);
    if profile.preferences.ssh_auth == "password"
        || !profile.preferences.identity_secret_reference.is_empty()
    {
        command
            .env(
                "SSH_ASKPASS",
                std::env::current_exe().map_err(|_| "SSH credential helper unavailable")?,
            )
            .env("SSH_ASKPASS_REQUIRE", "force")
            .env(crate::askpass::REFERENCE, &profile.secret_reference)
            .env(
                crate::askpass::KEY_REFERENCE,
                &profile.preferences.identity_secret_reference,
            )
            .env(crate::askpass::ONLY_SAVED, "1");
    }
    let bytes = bounded_process::exchange(&mut command, png, allowed, timeout)?;
    let out = String::from_utf8(bytes).map_err(|_| "Invalid remote image path")?;
    let path = out
        .strip_suffix('\n')
        .ok_or("Incomplete remote image receipt")?;
    if !path.starts_with('/')
        || path.chars().any(char::is_control)
        || !path.ends_with(&format!("/.cache/ruwac/images/{name}.png"))
        || std::path::Path::new(path)
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("Remote image destination rejected".into());
    }
    Ok(path.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn publication_checks_identity_hash_and_uses_no_replace() {
        let cmd = remote_command("image_generated", 123, &"a".repeat(64));
        assert!(cmd.contains("test -O"));
        assert!(cmd.contains("sha256sum --status"));
        assert!(cmd.contains("-eq 123"));
        assert!(cmd.contains("ln --"));
        assert!(!cmd.contains("chmod"));
    }
}
