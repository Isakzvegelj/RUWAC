//! Remove immutable ended pane IDs, preserving every window with a live process.
use serde_json::{Value, json};
use std::{collections::HashSet, io};

pub fn wipe(
    selected: Option<&str>,
    mut call: impl FnMut(&str, Value) -> io::Result<Value>,
) -> io::Result<Value> {
    let sessions = call("session.list", json!({}))?;
    let sessions = sessions
        .as_array()
        .ok_or_else(|| io::Error::other("Invalid session list"))?;
    let mut removed = 0;
    let mut windows = HashSet::new();
    let mut emptied = 0;
    for session in sessions {
        let Some(id) = session["id"]
            .as_str()
            .filter(|id| selected.is_none_or(|s| s == *id))
        else {
            continue;
        };
        let panes = call("pane.list", json!({"session":id}))?;
        let panes = panes
            .as_array()
            .ok_or_else(|| io::Error::other("Invalid pane list"))?;
        // IDs never respawn in place, including on older live daemons. A true
        // ended/restored record therefore cannot become another live process.
        let live: HashSet<_> = panes
            .iter()
            .filter(|p| p["ended"] != true && p["restored"] != true)
            .filter_map(|p| p["window"].as_str())
            .collect();
        for pane in panes
            .iter()
            .filter(|p| p["window"].as_str().is_some_and(|w| !live.contains(w)))
        {
            if pane["ended"] != true && pane["restored"] != true {
                continue;
            }
            let Some(pid) = pane["id"]
                .as_str()
                .filter(|id| ruwac_protocol::valid_id(id, "pane"))
            else {
                continue;
            };
            match call("pane.close", json!({"pane":pid,"if_ended":true})) {
                Ok(_) => {
                    removed += 1;
                    windows.insert(pane["window"].as_str().unwrap().to_owned());
                }
                Err(error) if stale(&error) => {}
                Err(error) => return Err(error),
            }
        }
        if !live.is_empty() {
            continue;
        }
        let remaining = call("session.list", json!({}))?;
        if remaining
            .as_array()
            .is_some_and(|rows| !rows.iter().any(|row| row["id"] == id))
        {
            emptied += 1;
        }
    }
    Ok(
        json!({"removed_panes":removed,"removed_windows":windows.len(),"removed_sessions":emptied,"live_processes_preserved":true}),
    )
}

pub fn stale(error: &io::Error) -> bool {
    serde_json::from_str::<Value>(&error.to_string()).is_ok_and(|value| value["code"] == "STALE_ID")
}
