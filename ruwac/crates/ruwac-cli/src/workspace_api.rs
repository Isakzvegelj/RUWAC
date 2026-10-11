//! Optional desktop control over authenticated local IPC; no GTK dependency or autostart.
use ruwac_protocol::{Error, read_frame, request, response, write_frame};
use serde_json::{Value, json};
use std::{io, path::Path, sync::mpsc, time::Duration};
pub fn run(socket: &Path, args: &[String]) -> io::Result<Value> {
    let bad = || {
        io::Error::other("workspace describe|list|activate --id tab_ID|focus --id view_ID [--json]")
    };
    let verb = args.first().ok_or_else(bad)?;
    let mut rest = &args[1..];
    if rest.last().is_some_and(|x| x == "--json") {
        rest = &rest[..rest.len() - 1];
    }
    let (method, params) = match verb.as_str() {
        "describe" | "list" if rest.is_empty() => (format!("workspace.{verb}"), json!({})),
        "activate" | "focus" if rest.len() == 2 && rest[0] == "--id" => {
            let kind = if verb == "activate" { "tab" } else { "view" };
            if !ruwac_protocol::valid_id(&rest[1], kind) {
                return Err(bad());
            }
            (format!("workspace.{verb}"), json!({"id":rest[1]}))
        }
        _ => return Err(bad()),
    };
    let r = request(&method, params);
    let id = r.id.clone();
    let path = socket.with_extension("gui.sock");
    let (tx, rx) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let result = (|| -> io::Result<Value> {
            let mut stream = super::startup::connect(&path)?;
            stream.set_read_timeout(Some(Duration::from_secs(3)))?;
            stream.set_write_timeout(Some(Duration::from_secs(1)))?;
            write_frame(&mut stream, &r)?;
            let v = read_frame(&mut stream)?;
            if v["version"] != 0
                || v["id"] != r.id
                || !(v.get("result").is_some_and(Value::is_object)
                    ^ v.get("error")
                        .is_some_and(|e| e["code"].is_string() && e["message"].is_string()))
            {
                return Err(io::Error::other(
                    "GUI response protocol/correlation rejected",
                ));
            }
            Ok(v)
        })();
        let _ = tx.send(result);
    });
    Ok(match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(v)) => v,
        Ok(Err(_)) => response(
            &id,
            Err(Error::new(
                "GUI_UNAVAILABLE",
                "Authenticated GUI endpoint unavailable; start ruwac gui for this daemon",
            )),
        ),
        Err(_) => response(
            &id,
            Err(Error::new(
                "GUI_TIMEOUT",
                "GUI endpoint deadline; no daemon or GUI was started",
            )),
        ),
    })
}
