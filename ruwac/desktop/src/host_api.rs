//! Read-only, explicit projections of the native machine library.
use ruwac_protocol::Error;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const PAGE_BYTES: usize = 128 * 1024;
fn invalid() -> Error {
    Error::new("INVALID_REQUEST", "Invalid host inventory options")
}
pub fn exchange(method: &str, params: &Value, mut rows: Vec<Value>) -> Result<Value, Error> {
    let p = params.as_object().ok_or_else(invalid)?;
    rows.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    if method == "host.query" {
        let id = p.get("id").and_then(Value::as_str).ok_or_else(invalid)?;
        if p.len() != 1 || !ruwac_protocol::valid_id(id, "endpoint") {
            return Err(invalid());
        }
        return rows
            .into_iter()
            .find(|r| r["id"] == id)
            .map(|r| json!({"endpoint":r}))
            .ok_or_else(|| {
                Error::new(
                    "STALE_PROFILE",
                    "Endpoint not present in the machine library",
                )
            });
    }
    if method != "host.list"
        || p.keys().any(|k| {
            ![
                "query",
                "group",
                "protocol",
                "favorites",
                "offset",
                "limit",
                "snapshot",
            ]
            .contains(&k.as_str())
        })
    {
        return Err(invalid());
    }
    let text = |key: &str, max: usize| -> Result<&str, Error> {
        match p.get(key) {
            None => Ok(""),
            Some(v) => v
                .as_str()
                .filter(|s| s.len() <= max && !s.chars().any(char::is_control))
                .ok_or_else(invalid),
        }
    };
    let query = text("query", 256)?.to_lowercase();
    let group = text("group", 1024)?;
    let protocol = text("protocol", 3)?;
    if !["", "SSH", "RDP"].contains(&protocol) {
        return Err(invalid());
    }
    let favorites = match p.get("favorites") {
        None => false,
        Some(v) => v.as_bool().ok_or_else(invalid)?,
    };
    let number = |key: &str, default: u64| match p.get(key) {
        None => Ok(default),
        Some(v) => v.as_u64().ok_or_else(invalid),
    };
    let offset = number("offset", 0)?;
    let limit = number("limit", 50)?;
    if offset > 1000 || !(1..=50).contains(&limit) {
        return Err(invalid());
    }
    rows.retain(|r| {
        let mut search = String::new();
        for k in ["name", "group", "server", "label", "network", "protocol"] {
            search.push_str(r[k].as_str().unwrap_or(""));
            search.push(' ');
        }
        if let Some(tags) = r["tags"].as_array() {
            for t in tags {
                search.push_str(t.as_str().unwrap_or(""));
                search.push(' ');
            }
        }
        (protocol.is_empty() || r["protocol"] == protocol)
            && (group.is_empty() || r["group"] == group)
            && (!favorites || r["favorite"] == true)
            && search.to_lowercase().contains(&query)
    });
    // Includes filters and every projected value, so metadata edits and filter
    // changes cannot silently produce a mixed collection across pages.
    let token = format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&json!([query, group, protocol, favorites, rows]))
                .map_err(|_| invalid())?
        )
    );
    let supplied = text("snapshot", 64)?;
    if (offset > 0 && supplied.is_empty())
        || (!supplied.is_empty()
            && (supplied.len() != 64 || !supplied.bytes().all(|b| b.is_ascii_hexdigit())))
    {
        return Err(invalid());
    }
    if !supplied.is_empty() && supplied != token {
        return Err(Error::new(
            "STALE_SNAPSHOT",
            "Machine inventory or filters changed; restart pagination",
        ));
    }
    let total = rows.len();
    if offset as usize > total {
        return Err(invalid());
    }
    let mut endpoints = Vec::new();
    let mut bytes = 1024;
    for r in rows.into_iter().skip(offset as usize).take(limit as usize) {
        let size = serde_json::to_vec(&r).map_err(|_| invalid())?.len() + 1;
        if bytes + size > PAGE_BYTES {
            break;
        }
        bytes += size;
        endpoints.push(r);
    }
    if endpoints.is_empty() && (offset as usize) < total {
        return Err(Error::new(
            "LIMIT",
            "Endpoint exceeds inventory page budget",
        ));
    }
    let next = offset as usize + endpoints.len();
    Ok(
        json!({"endpoints":endpoints,"total":total,"offset":offset,"next_offset":(next<total).then_some(next),"snapshot":token,"page_byte_limit":PAGE_BYTES,"source":"native-machine-library"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rows() -> Vec<Value> {
        (0..1000).map(|i|json!({"id":format!("endpoint_{i:032x}"),"name":format!("Machine {i}"),"group":"Lab","protocol":if i%2==0 {"SSH"}else{"RDP"},"server":"example.invalid","favorite":i%3==0,"tags":["unicode-č"],"network":"test"})).collect()
    }
    #[test]
    fn complete_inventory_pages_and_changed_values_or_filters_refuse_continuation() {
        let rows = rows();
        let mut offset = 0;
        let mut snapshot = Value::Null;
        let mut ids = std::collections::HashSet::new();
        loop {
            let mut p = json!({"offset":offset,"limit":50});
            if offset > 0 {
                p["snapshot"] = snapshot.clone();
            }
            let page = exchange("host.list", &p, rows.clone()).unwrap();
            snapshot = page["snapshot"].clone();
            assert_eq!(page["total"], 1000);
            assert!(serde_json::to_vec(&page).unwrap().len() <= PAGE_BYTES);
            for row in page["endpoints"].as_array().unwrap() {
                assert!(ids.insert(row["id"].as_str().unwrap().to_owned()));
            }
            match page["next_offset"].as_u64() {
                Some(n) => offset = n,
                None => break,
            }
        }
        assert_eq!(ids.len(), 1000);
        let mut changed = rows.clone();
        changed[999]["name"] = json!("changed");
        assert_eq!(
            exchange(
                "host.list",
                &json!({"offset":50,"snapshot":snapshot}),
                changed
            )
            .unwrap_err()
            .code,
            "STALE_SNAPSHOT"
        );
        assert_eq!(
            exchange(
                "host.list",
                &json!({"offset":50,"snapshot":snapshot,"favorites":true}),
                rows
            )
            .unwrap_err()
            .code,
            "STALE_SNAPSHOT"
        );
    }
    #[test]
    fn byte_budget_advances_by_emitted_rows_and_strict_options_apply() {
        let mut rows: Vec<_> = rows().into_iter().take(8).collect();
        for row in &mut rows {
            row["tags"] = json!(vec!["č".repeat(500); 64]);
        }
        let first = exchange("host.list", &json!({"limit":50}), rows.clone()).unwrap();
        let n = first["endpoints"].as_array().unwrap().len();
        assert!(n > 0 && n < 50);
        assert_eq!(first["next_offset"], n);
        assert!(serde_json::to_vec(&first).unwrap().len() <= PAGE_BYTES);
        for p in [
            json!({"offset":1}),
            json!({"limit":0}),
            json!({"limit":51}),
            json!({"favorites":"yes"}),
            json!({"protocol":"FTP"}),
            json!({"password":"PRIVATE"}),
            json!({"query":"\n"}),
            json!({"offset":1001}),
            json!({"snapshot":"bad"}),
        ] {
            assert_eq!(
                exchange("host.list", &p, rows.clone()).unwrap_err().code,
                "INVALID_REQUEST"
            );
        }
        let subset = exchange(
            "host.list",
            &json!({"protocol":"RDP","favorites":true,"query":"Č","group":"Lab"}),
            self::rows(),
        )
        .unwrap();
        assert_eq!(subset["total"], 167);
        assert_eq!(
            exchange(
                "host.query",
                &json!({"id":"endpoint_00000000000000000000000000000001"}),
                rows.clone()
            )
            .unwrap()["endpoint"]["name"],
            "Machine 1"
        );
        assert_eq!(
            exchange(
                "host.query",
                &json!({"id":ruwac_protocol::id("endpoint")}),
                rows
            )
            .unwrap_err()
            .code,
            "STALE_PROFILE"
        );
    }
}
