//! Bounded foreground-command metadata for status labels; never reads argv.
use serde_json::{Value, json};
use std::io;

pub fn refresh(frame: &mut Value, mut call: impl FnMut(&str, Value) -> io::Result<Value>) -> bool {
    let collect = (|| -> Option<Value> {
        let session = frame["session"].as_str()?;
        let windows = call("window.list", json!({"session":session})).ok()?;
        if windows.as_array()?.len() > 128 {
            return None;
        }
        let mut jobs = Vec::new();
        let mut offset = 0;
        let mut membership = None;
        for page in 0..128 {
            let result = call(
                "job.list",
                json!({"session":session,"offset":offset,"limit":100}),
            )
            .ok()?;
            if page != 0 && membership != result.get("membership").cloned() {
                return None;
            }
            membership = result.get("membership").cloned();
            let rows = result["jobs"].as_array()?;
            if rows.len() > 100 || jobs.len() + rows.len() > 128 {
                return None;
            }
            jobs.extend(rows.iter().cloned());
            if result["next_offset"].is_null() {
                break;
            }
            let next = result["next_offset"].as_u64()?;
            if page == 127 || next != offset + rows.len() as u64 || next <= offset || next > 128 {
                return None;
            }
            offset = next;
        }
        let mut commands = serde_json::Map::new();
        for window in windows.as_array()? {
            let id = window["id"].as_str()?;
            let pane = if frame["window"] == id {
                &frame["active_pane"]
            } else {
                &window["active_pane"]
            };
            let Some(job) = jobs
                .iter()
                .find(|job| job["pane"] == *pane && job["pane_ended"] != true)
            else {
                continue;
            };
            let command = job["foreground"]["command"]
                .as_str()
                .or_else(|| job["command"].as_str());
            if let Some(command) = command.filter(|s| !s.is_empty() && s.len() <= 256) {
                let name = std::path::Path::new(command)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(command);
                let name: String = name.chars().filter(|c| !c.is_control()).collect();
                if !name.is_empty() {
                    commands.insert(id.into(), json!(name));
                }
            }
        }
        Some(Value::Object(commands))
    })();
    let Some(commands) = collect else {
        // Older daemons can lack job metadata. Keep ordinary window labels and
        // never make terminal input depend on optional process information.
        return false;
    };
    let changed = frame["_commands"] != commands;
    frame["_commands"] = commands;
    changed
}

pub fn name(frame: &Value, id: &Value, stored: &Value) -> String {
    let stored = stored.as_str().unwrap_or("shell");
    match frame["_commands"]
        .get(id.as_str().unwrap_or(""))
        .and_then(Value::as_str)
    {
        Some(command) if matches!(stored, "shell" | "Terminal") || stored == command => {
            command.into()
        }
        Some(command) => format!("{stored} ({command})"),
        None => stored.into(),
    }
}
