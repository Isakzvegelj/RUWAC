//! Read-only diagnostics; explicit repair is restricted to the invoking foreground TTY.
use serde_json::{Value, json};
use std::{
    fs, io,
    os::unix::fs::{FileTypeExt, MetadataExt},
    path::Path,
    time::Duration,
};
fn check(checks: &mut Vec<Value>, name: &str, status: &str, message: &str) {
    checks.push(json!({"name":name,"status":status,"message":message}));
}
pub fn options(args: &[String]) -> io::Result<(bool, bool, bool)> {
    let (mut terminal, mut desktop, mut repair) = (false, false, false);
    let mut seen = std::collections::HashSet::new();
    for a in args {
        if !seen.insert(a) {
            return Err(io::Error::other("duplicate doctor option"));
        }
        match a.as_str() {
            "--terminal" => terminal = true,
            "--desktop" => desktop = true,
            "--repair" => repair = true,
            "--json" => (),
            _ => {
                return Err(io::Error::other(
                    "doctor [--desktop] [--terminal [--repair]] [--json]",
                ));
            }
        }
    }
    if repair && !terminal {
        return Err(io::Error::other("--repair requires --terminal"));
    }
    Ok((terminal, desktop, repair))
}
fn terminal(checks: &mut Vec<Value>, repair: bool) {
    if unsafe { libc::isatty(0) } != 1 {
        check(
            checks,
            "terminal",
            "FAIL",
            "No current stdin TTY. Run this command in the affected terminal.",
        );
        return;
    }
    let mut t: libc::termios = unsafe { std::mem::zeroed() };
    if unsafe { libc::tcgetattr(0, &mut t) } != 0 {
        check(
            checks,
            "terminal",
            "FAIL",
            "Cannot inspect current TTY attributes.",
        );
        return;
    }
    let foreground = unsafe { libc::tcgetpgrp(0) } == unsafe { libc::getpgrp() };
    if repair {
        if !foreground {
            check(
                checks,
                "terminal-repair",
                "FAIL",
                "Repair refused: this process is not the TTY foreground process group.",
            );
            return;
        }
        t.c_lflag |= libc::ICANON | libc::ECHO | libc::ISIG | libc::IEXTEN;
        t.c_iflag |= libc::ICRNL;
        t.c_oflag |= libc::OPOST | libc::ONLCR;
        // TCSANOW keeps queued input. Preserve control characters, baud rate and other flags.
        if unsafe { libc::tcsetattr(0, libc::TCSANOW, &t) } != 0 {
            check(
                checks,
                "terminal-repair",
                "FAIL",
                "Current TTY attribute repair failed.",
            );
            return;
        }
        check(
            checks,
            "terminal-repair",
            "PASS",
            "Restored canonical input, echo, signals and newline processing on this foreground TTY only; queued input and custom control characters preserved.",
        );
    }
    let sane = t.c_lflag & (libc::ICANON | libc::ECHO | libc::ISIG)
        == (libc::ICANON | libc::ECHO | libc::ISIG);
    check(
        checks,
        "terminal",
        if sane { "PASS" } else { "WARN" },
        if sane {
            "Current TTY has canonical input, echo and signals."
        } else {
            "Current TTY uses raw/no-echo attributes. This may be intentional; after a crashed client, explicitly use doctor --terminal --repair."
        },
    );
    check(
        checks,
        "terminal-emulator",
        "INFO",
        "Mouse reporting and emulator private modes are not inspected or reset. No terminal escape sequences are emitted.",
    );
}
fn daemon(checks: &mut Vec<Value>, socket: &Path) {
    if let Some(parent) = socket.parent() {
        match fs::symlink_metadata(parent) {
            Ok(m)
                if m.is_dir()
                    && m.uid() == unsafe { libc::geteuid() }
                    && m.mode() & 0o777 == 0o700 =>
            {
                ()
            }
            Ok(_) => {
                check(
                    checks,
                    "socket-parent",
                    "FAIL",
                    "Socket parent must be owned, nonsymlink and mode 0700. Select a private runtime directory; existing paths are preserved.",
                );
                return;
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                check(
                    checks,
                    "daemon",
                    "WARN",
                    "Private daemon directory does not exist. Use ruwac start when you want to create it.",
                );
                return;
            }
            Err(_) => {
                check(
                    checks,
                    "socket-parent",
                    "FAIL",
                    "Socket parent cannot be inspected.",
                );
                return;
            }
        }
    }
    match fs::symlink_metadata(socket) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            check(
                checks,
                "daemon",
                "WARN",
                "Daemon is not started at the selected socket. Use ruwac start; doctor does not start it.",
            );
            return;
        }
        Ok(m)
            if m.file_type().is_socket()
                && m.uid() == unsafe { libc::geteuid() }
                && m.mode() & 0o777 == 0o600 =>
        {
            ()
        }
        _ => {
            check(
                checks,
                "daemon-socket",
                "FAIL",
                "Selected path is not an owned mode-0600 Unix socket. It was not removed or replaced.",
            );
            return;
        }
    }
    let path = socket.to_path_buf();
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let socket = path.as_path();
        let result = (|| -> io::Result<()> {
            let mut s = crate::startup::connect(socket)?;
            s.set_read_timeout(Some(Duration::from_secs(1)))?;
            s.set_write_timeout(Some(Duration::from_secs(1)))?;
            let r = ruwac_protocol::request("health", json!({}));
            ruwac_protocol::write_frame(&mut s, &r)?;
            let v: Value = ruwac_protocol::read_frame(&mut s)?;
            if v["version"] != 0 || v["id"] != r.id || v["result"]["ok"] != true {
                return Err(io::Error::other("invalid daemon health response"));
            }
            Ok(())
        })();
        let _ = tx.send(result);
    });
    let result = rx.recv_timeout(Duration::from_secs(1)).unwrap_or_else(|_| {
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "daemon probe deadline",
        ))
    });
    match result {
        Ok(()) => check(
            checks,
            "daemon",
            "PASS",
            "Owned same-UID daemon responds with a compatible correlated health frame.",
        ),
        Err(e) => check(
            checks,
            "daemon",
            "FAIL",
            &format!(
                "Daemon probe failed ({:?}). Existing listener and sessions are preserved; verify the selected socket and endpoint version.",
                e.kind()
            ),
        ),
    }
}
pub fn run(socket: &Path, args: &[String]) -> io::Result<Value> {
    let (tty, desktop, repair) = options(args)?;
    let mut checks = Vec::new();
    if !tty || desktop {
        daemon(&mut checks, socket);
    }
    if tty {
        terminal(&mut checks, repair);
    }
    if desktop {
        let binary = std::env::current_exe()?;
        let entry = std::env::var_os("RUWAC_GUI_ENTRY")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| binary.parent().unwrap().join("ruwac-desktop"));
        let executable = fs::metadata(&entry).is_ok_and(|m| m.is_file() && m.mode() & 0o111 != 0);
        check(
            &mut checks,
            "desktop-binary",
            if executable { "PASS" } else { "FAIL" },
            if executable {
                "Optional native desktop executable exists; this check does not qualify its graphical runtime."
            } else {
                "Native desktop executable missing or not executable. Install/build the desktop component beside the CLI or set RUWAC_GUI_ENTRY."
            },
        );
        let display = std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty())
            || std::env::var_os("DISPLAY").is_some_and(|v| !v.is_empty());
        check(
            &mut checks,
            "desktop-display",
            if display { "PASS" } else { "FAIL" },
            if display {
                "A display environment is configured; compositor connectivity and physical input still require actual desktop acceptance."
            } else {
                "No desktop display environment. Launch from a graphical session; headless control still works."
            },
        );
        check(
            &mut checks,
            "secret-service-session",
            if std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some() {
                "INFO"
            } else {
                "WARN"
            },
            "RDP/SSH stored credentials require a desktop D-Bus Secret Service provider. Doctor never unlocks the keyring or reads credentials.",
        );
    }
    let ok = !checks.iter().any(|c| c["status"] == "FAIL");
    let mut v = json!({"version":0,"result":{"ok":ok,"checks":checks,"repair_requested":repair,"automatic_changes":false,"cli_version":env!("CARGO_PKG_VERSION")}});
    if !ok {
        v["error"] = json!({"code":"DIAGNOSTIC_FAILED","message":"One or more requested checks failed; inspect result.checks."});
    }
    Ok(v)
}
