//! User shell selection shared by the daemon and graphical client.
use std::{ffi::CStr, path::Path};
/// Apply terminal defaults even when an older daemon supplies the PTY.
/// Keep login-shell discovery literal; this wrapper executes on its endpoint.
pub fn terminal_argv(argv: Vec<String>) -> Vec<String> {
    let mut command = vec![
        "/usr/bin/env".into(),
        "-u".into(),
        "NO_COLOR".into(),
        "--".into(),
    ];
    command.extend(argv);
    command
}
pub fn default_terminal_argv() -> Result<Vec<String>, String> {
    default_argv().map(terminal_argv)
}
pub fn default_argv() -> Result<Vec<String>, String> {
    let configured = std::env::var("SHELL").ok().filter(|s| !s.is_empty());
    let shell = if let Some(shell) = configured {
        shell
    } else {
        let mut passwd: libc::passwd = unsafe { std::mem::zeroed() };
        let mut result = std::ptr::null_mut();
        let mut bytes = vec![0u8; 65536];
        let code = unsafe {
            libc::getpwuid_r(
                libc::geteuid(),
                &mut passwd,
                bytes.as_mut_ptr().cast(),
                bytes.len(),
                &mut result,
            )
        };
        if code != 0 || result.is_null() || passwd.pw_shell.is_null() {
            return Err("User login shell unavailable; set SHELL to an absolute executable".into());
        }
        unsafe { CStr::from_ptr(passwd.pw_shell) }
            .to_str()
            .map_err(|_| "User shell path is not UTF-8")?
            .to_owned()
    };
    login_argv(&shell)
}
fn login_argv(shell: &str) -> Result<Vec<String>, String> {
    if !Path::new(shell).is_absolute()
        || shell.len() > 4096
        || shell.chars().any(char::is_control)
        || !std::fs::metadata(shell).is_ok_and(|m| m.is_file())
    {
        return Err("User shell must be an absolute executable path".into());
    }
    Ok(vec![shell.into(), "-l".into()])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn user_shell_is_literal_and_loads_login_config() {
        assert_eq!(login_argv("/bin/bash").unwrap(), ["/bin/bash", "-l"]);
        assert!(login_argv("bash").is_err());
        assert!(login_argv("/bin/bash -c command").is_err());
        assert!(login_argv("/bin/bash\n").is_err());
    }
}
