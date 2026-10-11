mod startup;
use ruwac_protocol::{Request, read_frame, request, write_frame};
use serde_json::{Value, json};
use std::{
    io::{self, Read},
    path::PathBuf,
    time::Duration,
};
fn rpc(socket: &std::path::Path, r: &Request) -> io::Result<Value> {
    let mut s = startup::connect(socket)?;
    s.set_read_timeout(Some(Duration::from_secs(65)))?;
    s.set_write_timeout(Some(Duration::from_secs(10)))?;
    write_frame(&mut s, r)?;
    let v = read_frame(&mut s)?;
    if v["version"] != 0 || v["id"] != r.id {
        return Err(io::Error::other("daemon protocol/correlation rejected"));
    }
    if v.get("error").is_some() {
        println!("{}", v);
        return Err(io::Error::other("request failed"));
    }
    Ok(v)
}
fn main() {
    if let Err(e) = run() {
        eprintln!("ruwac: {e}");
        std::process::exit(1)
    }
}

fn named_request(a: &[String]) -> io::Result<Request> {
    let failure = || {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid or missing explicit target/options; use --help",
        )
    };
    if a.len() < 2 {
        return Err(failure());
    }
    let method = match (a[0].as_str(), a[1].as_str()) {
        ("session", "create") => "session.create",
        ("session", "list") => "session.list",
        ("window", "create") => "window.create",
        ("window", "list") => "window.list",
        ("pane", "list") => "pane.list",
        ("pane", "send-text") => "pane.sendText",
        ("pane", "send-key") => "pane.sendKey",
        ("pane", "capture") => "pane.capture",
        ("pane", "wait") => "pane.wait",
        ("pane", "split") => "pane.create",
        _ => return Err(failure()),
    };
    let allowed: &[&str] = match method {
        "session.create" => &["--name"],
        "session.list" => &[],
        "window.create" | "window.list" | "pane.list" => &["--session"],
        "pane.sendText" => &["--id", "--text", "--file"],
        "pane.sendKey" => &["--id", "--key"],
        "pane.capture" => &["--id", "--lines"],
        "pane.wait" => &["--id", "--contains", "--timeout-ms"],
        "pane.create" => &["--window"],
        _ => &[],
    };
    let mut options = std::collections::HashMap::new();
    let mut i = 2;
    while i < a.len() {
        if a[i] == "--json" || (a[i] == "--detach" && method == "session.create") {
            i += 1;
            continue;
        }
        if !allowed.contains(&a[i].as_str())
            || i + 1 >= a.len()
            || options.insert(a[i].as_str(), a[i + 1].as_str()).is_some()
        {
            return Err(failure());
        }
        i += 2;
    }
    let required = |k: &str| options.get(k).copied().ok_or_else(failure);
    let mut params = json!({});
    for (flag, key) in [
        ("--session", "session"),
        ("--id", "pane"),
        ("--window", "window"),
        ("--name", "name"),
        ("--key", "key"),
        ("--contains", "contains"),
    ] {
        if allowed.contains(&flag) {
            params[key] = json!(required(flag)?);
        }
    }
    for (flag, key) in [("--lines", "lines"), ("--timeout-ms", "timeout_ms")] {
        if let Some(value) = options.get(flag) {
            params[key] = json!(value.parse::<u64>().map_err(|_| failure())?);
        }
    }
    if method == "pane.sendText" {
        params["text"] = match (options.get("--text"), options.get("--file")) {
            (Some(text), None) => json!(text),
            (None, Some(path)) => {
                let mut bytes = Vec::new();
                std::fs::File::open(path)?
                    .take((ruwac_protocol::MAX_TEXT + 1) as u64)
                    .read_to_end(&mut bytes)?;
                if bytes.len() > ruwac_protocol::MAX_TEXT {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "text exceeds 256 KiB",
                    ));
                }
                json!(String::from_utf8(bytes).map_err(|_| failure())?)
            }
            _ => return Err(failure()),
        };
    }
    Ok(request(method, params))
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut a: Vec<_> = std::env::args().skip(1).collect();
    let mut socket = PathBuf::from(std::env::var("RUWAC_SOCKET").unwrap_or_else(|_| {
        format!(
            "{}/control.sock",
            std::env::var("XDG_RUNTIME_DIR")
                .map(|r| format!("{r}/ruwac"))
                .unwrap_or_else(|_| format!("/tmp/ruwac-{}", unsafe { libc::geteuid() }))
        )
    }));
    if a.first().map(String::as_str) == Some("--socket") {
        if a.len() < 3 {
            return Err("--socket requires path and command".into());
        }
        socket = PathBuf::from(a.remove(1));
        a.remove(0);
    }
    match a.first().map(String::as_str) {
        Some("--version") => println!("RUWAC {}", env!("CARGO_PKG_VERSION")),
        Some("session" | "window" | "pane") => println!("{}", rpc(&socket, &named_request(&a)?)?),
        Some("gui") => {
            use std::os::unix::process::CommandExt;
            let binary = std::env::current_exe()?;
            let installed = binary
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("ruwac-desktop");
            let entry = std::env::var_os("RUWAC_GUI_ENTRY")
                .map(PathBuf::from)
                .unwrap_or(installed);
            if !entry.is_file() {
                return Err(
                    "GUI unavailable: optional compiled ruwac-desktop is not installed".into(),
                );
            }
            if !a[1..]
                .iter()
                .any(|s| matches!(s.as_str(), "--help" | "--version"))
            {
                startup::ensure(&socket)?;
            }
            return Err(std::process::Command::new(entry)
                .arg("--socket")
                .arg(&socket)
                .args(&a[1..])
                .exec()
                .into());
        }
        Some("start") => {
            if a.len() != 1 {
                return Err("start takes only --socket before command".into());
            }
            let started = startup::ensure(&socket)?;
            println!(
                "{}",
                json!({"version":0,"result":{"ready":true,"start_requested":started}})
            );
        }
        Some("rdp" | "workspace") => {
            println!(
                "{}",
                json!({"error":{"code":"GUI_UNAVAILABLE","message":"RDP/workspace adapter is not implemented; no headless rendering fallback"}})
            );
            return Err("GUI service unavailable".into());
        }
        Some("serve") => {
            if a.len() != 1 {
                return Err("serve takes only --socket before command".into());
            }
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()?
                .block_on(ruwac_daemon::serve(&socket))?;
        }
        Some("attach") => {
            if a.len() != 3 || a[1] != "--session" {
                return Err("attach --session session_ID".into());
            }
            drop(startup::connect(&socket)?);
            ruwac_tui::attach(&socket, &a[2])?;
        }
        Some("rpc") => {
            if a.len() < 2 {
                return Err("rpc METHOD [--params JSON] [--idempotency KEY]".into());
            }
            let mut r = request(&a[1], json!({}));
            let mut i = 2;
            while i < a.len() {
                let value = a.get(i + 1).ok_or("option requires value")?;
                match a[i].as_str() {
                    "--params" => r.params = serde_json::from_str(value)?,
                    "--idempotency" => r.idempotency_key = Some(value.clone()),
                    _ => return Err("unknown rpc option".into()),
                }
                i += 2;
            }
            println!("{}", rpc(&socket, &r)?);
        }
        Some("request") => {
            let mut b = String::new();
            io::stdin()
                .take((ruwac_protocol::MAX_FRAME + 1) as u64)
                .read_to_string(&mut b)?;
            if b.len() > ruwac_protocol::MAX_FRAME {
                return Err("request exceeds 1 MiB".into());
            }
            let r: Request = serde_json::from_str(&b)?;
            println!("{}", rpc(&socket, &r)?);
        }
        Some("image-stage") => {
            if a.len() != 3 {
                return Err("image-stage PNG PRIVATE_DIRECTORY".into());
            }
            let r = ruwac_clipboard::stage_png(
                std::path::Path::new(&a[1]),
                std::path::Path::new(&a[2]),
            )
            .map_err(io::Error::other)?;
            println!(
                "{}",
                json!({"path":r.path,"mime":"image/png","bytes":r.bytes,"sha256":r.sha256,"width":r.width,"height":r.height,"attached":false})
            );
        }
        Some("health" | "version" | "capabilities" | "schema") => {
            println!("{}", rpc(&socket, &request(&a[0], json!({})))?);
        }
        Some("--help" | "help") | None => {
            println!(
                "RUWAC\n  ruwac --version\n  ruwac start  # starts/reuses a private persistent daemon\n  ruwac gui [--session session_ID]\n  ruwac session create --name NAME --detach --json\n  ruwac session list --json\n  ruwac pane list --session session_ID --json\n  ruwac pane send-text --id pane_ID --text TEXT --json\n  ruwac pane send-text --id pane_ID --file UTF8_FILE --json\n  ruwac pane send-key --id pane_ID --key Enter --json\n  ruwac pane capture --id pane_ID --lines 250 --json\n  ruwac pane wait --id pane_ID --contains TEXT --timeout-ms 5000 --json\n  ruwac --socket PATH serve\n  ruwac --socket PATH rpc METHOD --params JSON [--idempotency KEY]\n  ruwac --socket PATH request < request.json\n  ruwac --socket PATH attach --session session_ID\n  ruwac image-stage PNG PRIVATE_DIRECTORY\n  ruwac --socket PATH health|version|capabilities|schema\nRemote: pipe JSON into an existing authenticated ssh HOST ruwac request.\nNo desktop clipboard, image attachment, recovery after daemon loss or full Screen compatibility claim."
            );
        }
        _ => return Err("unknown command; use --help".into()),
    }
    Ok(())
}
