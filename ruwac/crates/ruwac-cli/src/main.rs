mod doctor;
mod image;
mod image_transfer;
mod remote;
mod startup;
mod workspace_api;
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
// Older live daemons are preserved: discover their schema before invoking a new method.
fn describe(call: impl Fn(&Request) -> io::Result<Value>) -> io::Result<Value> {
    let schema = call(&request("schema", json!({})))?;
    let methods = schema["result"]["methods"]
        .as_array()
        .ok_or_else(|| io::Error::other("daemon discovery schema rejected"))?;
    if methods.iter().any(|m| m == "describe") {
        return call(&request("describe", json!({})));
    }
    let capabilities = call(&request("capabilities", json!({})))?;
    let version = call(&request("version", json!({})))?;
    Ok(
        json!({"version":0,"id":ruwac_protocol::id("request"),"result":{"protocol":0,"native_describe":false,"schema":schema["result"],"capabilities":capabilities["result"],"version":version["result"],"transport":{"local":"same-UID authenticated Unix socket","remote":"authenticated OpenSSH CLI --host","gui":"independent optional service"}}}),
    )
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
        "pane.capture" => &["--id", "--lines", "--buffer"],
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
    if let Some(value) = options.get("--buffer") {
        if !matches!(*value, "active" | "primary") {
            return Err(failure());
        }
        params["buffer"] = json!(value);
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

fn job_request(args: &[String]) -> io::Result<Request> {
    let bad = || {
        io::Error::other(
            "job list [--session ID] [--query TEXT] [--offset N] [--limit N] | job query --id pane_ID [--json]",
        )
    };
    let verb = args.first().ok_or_else(bad)?;
    let mut params = json!({});
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--json" && i + 1 == args.len() {
            i += 1;
            continue;
        }
        if i + 1 >= args.len() {
            return Err(bad());
        }
        let key = match (verb.as_str(), args[i].as_str()) {
            ("list", "--session") => "session",
            ("list", "--query") => "query",
            ("list", "--offset") => "offset",
            ("list", "--limit") => "limit",
            ("query", "--id") => "pane",
            _ => return Err(bad()),
        };
        if params.get(key).is_some() {
            return Err(bad());
        }
        params[key] = if matches!(key, "offset" | "limit") {
            json!(args[i + 1].parse::<u64>().map_err(|_| bad())?)
        } else {
            json!(args[i + 1])
        };
        i += 2;
    }
    if verb == "query" && params.get("pane").is_none() {
        return Err(bad());
    }
    if !matches!(verb.as_str(), "list" | "query") {
        return Err(bad());
    }
    Ok(request(&format!("job.{verb}"), params))
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
    let mut host = None;
    let mut remote_binary = None;
    let mut explicit_socket = None;
    while a
        .first()
        .is_some_and(|v| matches!(v.as_str(), "--socket" | "--host" | "--remote-binary"))
    {
        if a.len() < 3 {
            return Err("global option requires a value and command".into());
        }
        let option = a.remove(0);
        let value = a.remove(0);
        match option.as_str() {
            "--socket" => {
                if explicit_socket.replace(PathBuf::from(&value)).is_some() {
                    return Err("duplicate --socket".into());
                }
                socket = PathBuf::from(value);
            }
            "--host" => {
                if host.replace(value).is_some() {
                    return Err("duplicate --host".into());
                }
            }
            "--remote-binary" => {
                if remote_binary.replace(value).is_some() {
                    return Err("duplicate --remote-binary".into());
                }
            }
            _ => unreachable!(),
        }
    }
    if remote_binary.is_some() && host.is_none() {
        return Err("--remote-binary requires --host".into());
    }
    if a.first().is_some_and(|command| command == "update") {
        if host.is_some() || explicit_socket.is_some() {
            return Err("run updates locally on each machine, without --host or --socket".into());
        }
        let status = std::process::Command::new("python3")
            .arg("-c")
            .arg(include_str!("../../../../scripts/update-user.py"))
            .arg("--current")
            .arg(env!("CARGO_PKG_VERSION"))
            .args(&a[1..])
            .status()?;
        if !status.success() {
            return Err("update failed; see the diagnostics above".into());
        }
        return Ok(());
    }
    let remote = host.map(|host| remote::Remote {
        host,
        binary: remote_binary.unwrap_or_else(|| "ruwac".into()),
        socket: explicit_socket,
    });
    if let Some(remote) = &remote {
        remote.validate()?;
        match a.first().map(String::as_str) {
            Some("doctor") => {
                doctor::options(&a[1..])?;
                if a[1..].iter().any(|s| s == "--repair") {
                    return Err("remote terminal repair requires running doctor directly in its foreground TTY".into());
                }
                let v = remote.doctor(&a[1..])?;
                println!("{v}");
                if v["result"]["ok"] != true {
                    return Err("remote diagnostics failed".into());
                }
                return Ok(());
            }
            Some("start") => {
                if a.len() != 1 {
                    return Err("start takes no command options".into());
                }
                println!("{}", remote.start()?);
                return Ok(());
            }
            Some("attach") => {
                if a.len() != 3 || a[1] != "--session" {
                    return Err("attach --session session_ID".into());
                }
                remote.attach(&a[2])?;
                return Ok(());
            }
            Some("workspace" | "rdp" | "host") => {
                let req = workspace_api::parse(&a[0], &a[1..])?;
                let v = remote.gui(&req)?;
                println!("{v}");
                if v.get("error").is_some() {
                    return Err("remote GUI operation failed".into());
                }
                return Ok(());
            }
            Some("image-attach") => {
                if a.len() != 4 {
                    return Err(
                        "--host HOST image-attach pane_ID LOCAL_PNG REMOTE_PRIVATE_DIRECTORY"
                            .into(),
                    );
                }
                println!("{}", image_transfer::send(remote, &a[1], &a[2], &a[3])?);
                return Ok(());
            }
            Some("gui" | "serve" | "image-stage" | "image-receive" | "gui-request") => return Err(
                "--host supports authenticated control; launch the GUI directly on its endpoint"
                    .into(),
            ),
            _ => (),
        }
    }
    let call = |request: &Request| match &remote {
        Some(remote) => remote.rpc(request),
        None => rpc(&socket, request),
    };
    match a.first().map(String::as_str) {
        Some("doctor") => {
            let v = doctor::run(&socket, &a[1..])?;
            println!("{v}");
            if v["result"]["ok"] != true {
                return Err("requested diagnostics failed".into());
            }
        }
        Some("--version") => println!("RUWAC {}", env!("CARGO_PKG_VERSION")),
        Some("session" | "window" | "pane") => println!("{}", call(&named_request(&a)?)?),
        Some("job") => println!("{}", call(&job_request(&a[1..])?)?),
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
        Some("workspace" | "rdp" | "host") => {
            let req = workspace_api::parse(&a[0], &a[1..])?;
            let v = workspace_api::exchange(&socket, &req)?;
            println!("{v}");
            if v.get("error").is_some() {
                return Err("GUI operation failed".into());
            }
        }
        Some("gui-request") => {
            if a.len() != 1 {
                return Err("gui-request takes one JSON stdin request and no options".into());
            }
            let mut bytes = Vec::new();
            io::stdin()
                .take(ruwac_protocol::MAX_FRAME as u64 + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > ruwac_protocol::MAX_FRAME {
                return Err("GUI request exceeds frame cap".into());
            }
            let req: Request = serde_json::from_slice(&bytes)?;
            let v = workspace_api::exchange(&socket, &req)?;
            println!("{v}");
            if v.get("error").is_some() {
                return Err("GUI operation failed".into());
            }
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
            println!("{}", call(&r)?);
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
            println!("{}", call(&r)?);
        }
        Some("image-receive") => {
            if a.len() != 1 {
                return Err("image-receive reads a bounded SSH binary stdin transfer".into());
            }
            println!("{}", image_transfer::receive(&call)?);
        }
        Some("image-attach") => {
            if a.len() != 4 {
                return Err(
                    "image-attach pane_ID PNG PRIVATE_DIRECTORY (local Codex draft only)".into(),
                );
            }
            println!("{}", image::attach(&a[1], &a[2], &a[3], &call)?);
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
        Some("describe") => {
            if a.len() > 2 || (a.len() == 2 && a[1] != "--json") {
                return Err("describe takes only optional --json".into());
            }
            println!("{}", describe(call)?);
        }
        Some("health" | "version" | "capabilities" | "schema") => {
            if a.len() > 2 || (a.len() == 2 && a[1] != "--json") {
                return Err("discovery takes only optional --json".into());
            }
            println!("{}", call(&request(&a[0], json!({})))?);
        }
        Some("--help" | "help") | None => {
            println!(
                "Updates (Fedora 44): ruwac update --check | --install | --rollback | --auto-check on|off|status.\nSource builds require development dependencies; --headless retains a CLI-only installation. No running processes are restarted."
            );
            println!(
                "RUWAC\n  ruwac job list [--session session_ID] [--query TEXT] --json\n  ruwac job query --id pane_ID --json\n  ruwac workspace describe|list [--json]\n  ruwac host list [--query TEXT] [--group GROUP] [--protocol SSH|RDP] [--favorites] [--limit 1..50] [--offset N --snapshot TOKEN] --json\n  ruwac host query --id endpoint_ID --json\n  ruwac rdp list --json\n  ruwac rdp query|reconnect|disconnect --id view_ID --json\n  ruwac rdp open --profile endpoint_ID [--activate] --json\n  ruwac workspace activate --id tab_ID --json\n  ruwac workspace focus --id view_ID --json\n  ruwac doctor [--desktop] [--terminal [--repair]] [--json]\n  ruwac --version\n  ruwac start  # starts/reuses a private persistent daemon\n  ruwac gui [--session session_ID]\n  ruwac session create --name NAME --detach --json\n  ruwac session list --json\n  ruwac pane list --session session_ID --json\n  ruwac pane send-text --id pane_ID --text TEXT --json\n  ruwac pane send-text --id pane_ID --file UTF8_FILE --json\n  ruwac pane send-key --id pane_ID --key Enter --json\n  ruwac pane capture --id pane_ID --lines 250 [--buffer active|primary] --json\n  ruwac pane wait --id pane_ID --contains TEXT --timeout-ms 5000 --json\n  ruwac --socket PATH serve\n  ruwac --socket PATH rpc METHOD --params JSON [--idempotency KEY]\n  ruwac --socket PATH request < request.json\n  ruwac --socket PATH attach --session session_ID\n  ruwac image-stage PNG PRIVATE_DIRECTORY\n  ruwac --socket PATH image-attach pane_ID PNG PRIVATE_DIRECTORY\n  ruwac --socket PATH health|version|capabilities|schema|describe\nRemote: ruwac --host SSH_HOST [--remote-binary PATH] [--socket REMOTE_PATH] COMMAND.\n  ruwac --host SSH_HOST describe\n  ruwac --host SSH_HOST pane send-text --id pane_ID --file LOCAL_UTF8_FILE\n  ruwac --host SSH_HOST attach --session session_ID\n  ruwac --host SSH_HOST image-attach pane_ID LOCAL_PNG REMOTE_PRIVATE_DIRECTORY\nHost/workspace/RDP commands address an already-running remote GUI with its explicit permissions. Uses existing SSH keys/config and verified host keys; no credential or agent forwarding. Without explicit --socket, the remote CLI selects its own private default.\nImage attachment: explicit local or SSH Codex draft; no automatic submission. No desktop clipboard or full Screen compatibility claim."
            );
        }
        _ => return Err("unknown command; use --help".into()),
    }
    Ok(())
}
