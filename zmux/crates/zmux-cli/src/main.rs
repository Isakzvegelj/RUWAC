use serde_json::{Value, json};
use std::{
    io::{self, Read},
    os::unix::net::UnixStream,
    path::PathBuf,
    time::Duration,
};
use zmux_protocol::{Request, read_frame, request, write_frame};
fn rpc(socket: &std::path::Path, r: &Request) -> io::Result<Value> {
    let mut s = UnixStream::connect(socket)?;
    s.set_read_timeout(Some(Duration::from_secs(65)))?;
    s.set_write_timeout(Some(Duration::from_secs(10)))?;
    write_frame(&mut s, r)?;
    let v = read_frame(&mut s)?;
    if v.get("error").is_some() {
        println!("{}", v);
        return Err(io::Error::other("request failed"));
    }
    Ok(v)
}
fn main() {
    if let Err(e) = run() {
        eprintln!("zmux: {e}");
        std::process::exit(1)
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut a: Vec<_> = std::env::args().skip(1).collect();
    let mut socket = PathBuf::from(std::env::var("ZMUX_SOCKET").unwrap_or_else(|_| {
        format!(
            "{}/control.sock",
            std::env::var("XDG_RUNTIME_DIR")
                .map(|r| format!("{r}/zmux"))
                .unwrap_or_else(|_| format!("/tmp/zmux-{}", unsafe { libc::geteuid() }))
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
        Some("serve") => {
            if a.len() != 1 {
                return Err("serve takes only --socket before command".into());
            }
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()?
                .block_on(zmux_daemon::serve(&socket))?;
        }
        Some("attach") => {
            if a.len() != 3 || a[1] != "--session" {
                return Err("attach --session session_ID".into());
            }
            zmux_tui::attach(&socket, &a[2])?;
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
                .take((zmux_protocol::MAX_FRAME + 1) as u64)
                .read_to_string(&mut b)?;
            if b.len() > zmux_protocol::MAX_FRAME {
                return Err("request exceeds 1 MiB".into());
            }
            let r: Request = serde_json::from_str(&b)?;
            println!("{}", rpc(&socket, &r)?);
        }
        Some("image-stage") => {
            if a.len() != 3 {
                return Err("image-stage PNG PRIVATE_DIRECTORY".into());
            }
            let r =
                zmux_clipboard::stage_png(std::path::Path::new(&a[1]), std::path::Path::new(&a[2]))
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
                "zMux spike\n  zmux --socket PATH serve\n  zmux --socket PATH rpc METHOD --params JSON [--idempotency KEY]\n  zmux --socket PATH request < request.json\n  zmux --socket PATH attach --session session_ID\n  zmux image-stage PNG PRIVATE_DIRECTORY\n  zmux --socket PATH health|version|capabilities|schema\nRemote: pipe JSON into an existing authenticated ssh HOST zmux request.\nNo desktop clipboard, image attachment, recovery after daemon loss or full Screen compatibility claim."
            );
        }
        _ => return Err("unknown command; use --help".into()),
    }
    Ok(())
}
