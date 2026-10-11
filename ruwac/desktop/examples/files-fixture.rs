//! Explicit fixture driver for the production SFTP and Secret Service adapters.
#[path = "../src/credentials.rs"]
mod credentials;
#[path = "../src/files.rs"]
mod files;
#[path = "../src/profiles.rs"]
mod profiles;
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64},
    },
    time::Duration,
};
fn main() {
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
        if args[1] == "list" {
            let entries = files::list(&p, Path::new(&args[3]))?;
            return Ok(
                serde_json::json!({"entries":entries.iter().map(|e|&e.name).collect::<Vec<_>>()}),
            );
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
