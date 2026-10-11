//! Optional desktop control over authenticated local IPC; no GTK dependency or autostart.
use ruwac_protocol::{Error, read_frame, request, response, write_frame};
use serde_json::{Value, json};
use std::{io, path::Path, sync::mpsc, time::Duration};
pub fn parse(namespace: &str, args: &[String]) -> io::Result<ruwac_protocol::Request> {
    let bad=||io::Error::other("Explicit workspace/RDP operation and target required; use --help");
    let verb=args.first().ok_or_else(bad)?;
    let mut rest=&args[1..];
    if rest.last().is_some_and(|x|x=="--json"){rest=&rest[..rest.len()-1];}
    let params=match (namespace,verb.as_str()) {
        ("workspace","describe"|"list")|("rdp","list") if rest.is_empty()=>json!({}),
        ("workspace","activate"|"focus")|("rdp","query"|"reconnect"|"disconnect") if rest.len()==2&&rest[0]=="--id"=>{
            let kind=if namespace=="workspace"&&verb=="activate" {"tab"}else{"view"};
            if !ruwac_protocol::valid_id(&rest[1],kind){return Err(bad());}
            json!({"id":rest[1]})
        },
        ("rdp","open") if (rest.len()==2||rest.len()==3&&rest[2]=="--activate")&&rest[0]=="--profile"=>{
            if !ruwac_protocol::valid_id(&rest[1],"endpoint"){return Err(bad());}
            json!({"profile":rest[1],"activate":rest.len()==3})
        },
        _=>return Err(bad()),
    };
    Ok(request(&format!("{namespace}.{verb}"),params))
}
pub fn exchange(socket:&Path,r:&ruwac_protocol::Request)->io::Result<Value> {
    let r=r.clone();
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

#[cfg(test)] mod tests {
 use super::*;
 #[test] fn gui_commands_require_typed_explicit_targets_and_exact_options() {
  let view=ruwac_protocol::id("view");let profile=ruwac_protocol::id("endpoint");
  assert_eq!(parse("rdp",&["query".into(),"--id".into(),view.clone()]).unwrap().params["id"],view);
  assert_eq!(parse("rdp",&["open".into(),"--profile".into(),profile,"--activate".into(),"--json".into()]).unwrap().params["activate"],true);
  assert!(parse("rdp",&["disconnect".into()]).is_err());
  assert!(parse("rdp",&["open".into(),"--profile".into(),view]).is_err());
  assert!(parse("workspace",&["list".into(),"extra".into()]).is_err());
 }
}
