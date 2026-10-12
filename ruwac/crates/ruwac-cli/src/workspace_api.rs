//! Optional desktop control over authenticated local IPC; no GTK dependency or autostart.
use ruwac_protocol::{Error, read_frame, request, response, write_frame};
use serde_json::{Value, json};
use std::{io, path::Path, sync::mpsc, time::Duration};
pub fn parse(namespace: &str, args: &[String]) -> io::Result<ruwac_protocol::Request> {
    let bad =
        || io::Error::other("Explicit workspace/RDP operation and target required; use --help");
    if namespace == "host" {
        return host_request(args);
    }
    let verb = args.first().ok_or_else(bad)?;
    let mut rest = &args[1..];
    if rest.last().is_some_and(|x| x == "--json") {
        rest = &rest[..rest.len() - 1];
    }
    let params = match (namespace, verb.as_str()) {
        ("workspace", "describe" | "list") | ("rdp", "list") if rest.is_empty() => json!({}),
        ("workspace", "activate" | "focus") | ("rdp", "query" | "reconnect" | "disconnect")
            if rest.len() == 2 && rest[0] == "--id" =>
        {
            let kind = if namespace == "workspace" && verb == "activate" {
                "tab"
            } else {
                "view"
            };
            if !ruwac_protocol::valid_id(&rest[1], kind) {
                return Err(bad());
            }
            json!({"id":rest[1]})
        }
        ("rdp", "open")
            if (rest.len() == 2 || rest.len() == 3 && rest[2] == "--activate")
                && rest[0] == "--profile" =>
        {
            if !ruwac_protocol::valid_id(&rest[1], "endpoint") {
                return Err(bad());
            }
            json!({"profile":rest[1],"activate":rest.len()==3})
        }
        _ => return Err(bad()),
    };
    Ok(request(&format!("{namespace}.{verb}"), params))
}
pub fn exchange(socket: &Path, r: &ruwac_protocol::Request) -> io::Result<Value> {
    let r = r.clone();
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gui_commands_require_typed_explicit_targets_and_exact_options() {
        let view = ruwac_protocol::id("view");
        let profile = ruwac_protocol::id("endpoint");
        assert_eq!(
            parse("rdp", &["query".into(), "--id".into(), view.clone()])
                .unwrap()
                .params["id"],
            view
        );
        assert_eq!(
            parse(
                "rdp",
                &[
                    "open".into(),
                    "--profile".into(),
                    profile,
                    "--activate".into(),
                    "--json".into()
                ]
            )
            .unwrap()
            .params["activate"],
            true
        );
        assert!(parse("rdp", &["disconnect".into()]).is_err());
        assert!(parse("rdp", &["open".into(), "--profile".into(), view]).is_err());
        assert!(parse("workspace", &["list".into(), "extra".into()]).is_err());
    }
}

fn host_request(args: &[String]) -> io::Result<ruwac_protocol::Request> {
    let bad = || {
        io::Error::other(
            "host list [--query TEXT] [--group GROUP] [--protocol SSH|RDP] [--favorites] [--limit 1..50] [--offset N --snapshot TOKEN] | host query --id endpoint_ID [--json]",
        )
    };
    let verb = args.first().ok_or_else(bad)?;
    if !matches!(verb.as_str(), "list" | "query") {
        return Err(bad());
    }
    let mut params = serde_json::Map::new();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--json" && i + 1 == args.len() {
            break;
        }
        let key = match args[i].as_str() {
            "--query" => "query",
            "--group" => "group",
            "--protocol" => "protocol",
            "--favorites" => "favorites",
            "--limit" => "limit",
            "--offset" => "offset",
            "--snapshot" => "snapshot",
            "--id" => "id",
            _ => return Err(bad()),
        };
        if (verb == "query") != (key == "id") || params.contains_key(key) {
            return Err(bad());
        }
        let value = if key == "favorites" {
            i += 1;
            json!(true)
        } else {
            let text = args.get(i + 1).ok_or_else(bad)?;
            i += 2;
            if matches!(key, "limit" | "offset") {
                let n = text.parse::<u64>().map_err(|_| bad())?;
                if key == "limit" && !(1..=50).contains(&n) || key == "offset" && n > 1000 {
                    return Err(bad());
                }
                json!(n)
            } else {
                let cap = if key == "query" {
                    256
                } else if key == "snapshot" {
                    64
                } else {
                    1024
                };
                if text.len() > cap
                    || text.chars().any(char::is_control)
                    || key == "protocol" && !matches!(text.as_str(), "SSH" | "RDP")
                    || key == "id" && !ruwac_protocol::valid_id(text, "endpoint")
                {
                    return Err(bad());
                }
                if key == "snapshot"
                    && (text.len() != 64 || !text.bytes().all(|b| b.is_ascii_hexdigit()))
                {
                    return Err(bad());
                }
                json!(text)
            }
        };
        params.insert(key.into(), value);
    }
    if verb == "query" && !params.contains_key("id")
        || params
            .get("offset")
            .and_then(Value::as_u64)
            .is_some_and(|n| n > 0)
            && !params.contains_key("snapshot")
    {
        return Err(bad());
    }
    Ok(request(&format!("host.{verb}"), Value::Object(params)))
}
