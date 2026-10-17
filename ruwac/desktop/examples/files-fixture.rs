//! Explicit fixture driver for the production SFTP and Secret Service adapters.
#[path = "../src/askpass.rs"]
mod askpass;
#[path = "../src/credentials.rs"]
mod credentials;
#[path = "../src/files.rs"]
mod files;
#[path = "../src/files_resume.rs"]
mod files_resume;
#[path = "../src/files_rsync.rs"]
mod files_rsync;
#[path = "../src/profiles.rs"]
mod profiles;
#[path = "../src/rdp_trust.rs"]
mod rdp_trust;
#[path = "../src/remote_paste_upload.rs"]
mod remote_paste_upload;
#[path = "../src/sidebar_shell.rs"]
mod sidebar_shell;
#[path = "../src/ssh_jump.rs"]
mod ssh_jump;
#[path = "../src/ssh_keys.rs"]
mod ssh_keys;
#[path = "../src/ssh_probe.rs"]
mod ssh_probe;
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64},
    },
    time::Duration,
};
fn main() {
    if let Some(status) = askpass::run_if_requested() {
        std::process::exit(status);
    }
    let args: Vec<_> = std::env::args().collect();
    let result = (|| -> Result<serde_json::Value, String> {
        if args.get(1).map(String::as_str) == Some("credentials") {
            let reference = ruwac_protocol::id("fixture-secret");
            credentials::set(&reference, "synthetic-keyring-fixture-only".into())?;
            let matched = credentials::lookup(&reference)? == "synthetic-keyring-fixture-only";
            credentials::set(&reference, "synthetic-replacement-only".into())?;
            let replaced = credentials::lookup(&reference)? == "synthetic-replacement-only";
            credentials::forget(&reference)?;
            let forgotten = credentials::lookup(&reference).is_err();
            return Ok(
                serde_json::json!({"matched":matched,"replaced":replaced,"forgotten":forgotten,"provider":"real Secret Service via secret-tool"}),
            );
        }
        let p: profiles::Profile = serde_json::from_slice(
            &std::fs::read(args.get(2).ok_or("profile required")?)
                .map_err(|_| "cannot read fixture profile")?,
        )
        .map_err(|_| "invalid fixture profile")?;
        p.validate()?;
        if args[1] == "ssh-image-paste" {
            let bytes = std::fs::read(args.get(3).ok_or("PNG input required")?)
                .map_err(|_| "PNG input unavailable")?;
            let allowed = Arc::new(AtomicBool::new(true));
            if let Some(ms) = args.get(4).and_then(|v| v.parse::<u64>().ok()) {
                let ticket = allowed.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(ms));
                    ticket.store(false, std::sync::atomic::Ordering::Release);
                });
            }
            return remote_paste_upload::upload(&p, &bytes, &allowed, Duration::from_secs(10))
                .map(|path| serde_json::json!({"path":path}));
        }
        if args[1] == "ssh-argv" {
            return Ok(serde_json::json!({"argv": profiles::ssh_argv(&p)?}));
        }
        if args[1] == "ssh-access" {
            let keys = ssh_probe::identities();
            let outcome = ssh_probe::probe(&p, &keys);
            return Ok(serde_json::json!({
                "access": outcome.access,
                "accepted_key": outcome.accepted_key,
                "description": outcome.describe(),
                "inputs": ssh_probe::inputs(&p, &keys),
                "identities": keys.iter().map(|k| k.fingerprint.clone()).collect::<Vec<_>>(),
            }));
        }
        if args[1] == "rsync-pending" {
            return serde_json::to_value(files_rsync::pending(&p)?)
                .map_err(|_| "Cannot encode recovery jobs".into());
        }
        if args[1] == "rsync-recover" {
            return files_rsync::recover(&p, &args[3], &AtomicBool::new(false))
                .map(|s| serde_json::json!({"status":s}));
        }
        if matches!(args[1].as_str(), "rsync-upload" | "rsync-download") {
            let cancel = Arc::new(AtomicBool::new(false));
            let progress = Arc::new(AtomicU64::new(0));
            if let Some(threshold) = args.get(6).and_then(|s| s.parse::<u64>().ok()) {
                let (c, b) = (cancel.clone(), progress.clone());
                std::thread::spawn(move || {
                    while b.load(std::sync::atomic::Ordering::Relaxed) < threshold {
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    c.store(true, std::sync::atomic::Ordering::Relaxed);
                });
            }
            let observe_phase = args.get(7).cloned();
            return serde_json::to_value(files_rsync::transfer_observed(
                &p,
                Path::new(&args[3]),
                Path::new(&args[4]),
                args[1] == "rsync-upload",
                args.get(5).map(String::as_str) == Some("overwrite"),
                &cancel,
                progress,
                move |id, phase| {
                    if observe_phase.as_deref() == Some(phase) {
                        use std::io::Write;
                        println!("{}", serde_json::json!({"checkpoint":phase,"id":id}));
                        std::io::stdout()
                            .flush()
                            .map_err(|_| "Fixture output unavailable")?;
                        unsafe {
                            libc::raise(libc::SIGSTOP);
                        }
                    }
                    Ok(())
                },
            )?)
            .map_err(|_| "Cannot encode fixture receipt".into());
        }
        if args[1] == "list" {
            let entries = files::list(&p, Path::new(&args[3]))?;
            return Ok(
                serde_json::json!({"entries":entries.iter().map(|e|&e.name).collect::<Vec<_>>()}),
            );
        }
        if args[1] == "pending" {
            return serde_json::to_value(files_resume::list(&p)?).map_err(|e| e.to_string());
        }
        if args[1] == "discard" {
            files_resume::discard(&p, &args[3], &AtomicBool::new(false))?;
            return Ok(serde_json::json!({"discarded":true}));
        }
        if matches!(
            args[1].as_str(),
            "resume-upload" | "resume-download" | "resume"
        ) {
            let cancel = Arc::new(AtomicBool::new(false));
            let progress = Arc::new(AtomicU64::new(0));
            if let Some(threshold) = args
                .get(if args[1] == "resume" { 4 } else { 6 })
                .and_then(|s| s.parse::<u64>().ok())
            {
                let (c, b) = (cancel.clone(), progress.clone());
                std::thread::spawn(move || {
                    while b.load(std::sync::atomic::Ordering::Relaxed) < threshold {
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    c.store(true, std::sync::atomic::Ordering::Relaxed);
                });
            }
            let receipt = if args[1] == "resume" {
                files_resume::resume(&p, &args[3], &cancel, &progress)
            } else {
                files_resume::start(
                    &p,
                    Path::new(&args[3]),
                    Path::new(&args[4]),
                    args[1] == "resume-upload",
                    args.get(5).map(String::as_str) == Some("overwrite"),
                    &cancel,
                    &progress,
                )
            }?;
            return serde_json::to_value(receipt).map_err(|e| e.to_string());
        }
        let cancel = Arc::new(AtomicBool::new(false));
        if let Some(ms) = args.get(6).and_then(|s| s.parse::<u64>().ok()) {
            let c = cancel.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(ms));
                c.store(true, std::sync::atomic::Ordering::Relaxed);
            });
        }
        let progress = AtomicU64::new(0);
        let start = std::time::Instant::now();
        let hash = files::transfer(
            &p,
            Path::new(&args[3]),
            Path::new(&args[4]),
            args[1] == "upload",
            args.get(5).map(String::as_str) == Some("overwrite"),
            &cancel,
            &progress,
        )?;
        Ok(
            serde_json::json!({"sha256":hash,"bytes":progress.load(std::sync::atomic::Ordering::Relaxed),"elapsed_ms":start.elapsed().as_millis()}),
        )
    })();
    println!(
        "{}",
        match result {
            Ok(v) => serde_json::json!({"ok":true,"result":v}),
            Err(e) => serde_json::json!({"ok":false,"error":e}),
        }
    );
}
