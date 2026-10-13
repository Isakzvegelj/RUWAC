//! Metadata-only observer feed. Cursors never silently skip an overwritten history.
use ruwac_protocol::{Error, Result};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    time::{Duration, Instant},
};
const TYPES: [&str; 4] = ["tab", "focus", "layout", "rdp"];
#[derive(Default, Clone, PartialEq)]
pub struct Snapshot {
    pub tabs: BTreeMap<String, Value>,
    pub focus: Option<String>,
    pub selected: Option<String>,
    pub active: bool,
    pub rdp: BTreeMap<String, (String, bool)>,
}
struct Subscription {
    uid: u32,
    types: HashSet<String>,
    seen: Instant,
}
pub struct Hub {
    instance: String,
    seq: u64,
    ring: VecDeque<Value>,
    subscriptions: HashMap<String, Subscription>,
    last: Option<Snapshot>,
}
fn invalid() -> Error {
    Error::new("INVALID_REQUEST", "Invalid GUI event parameters")
}
impl Hub {
    pub fn new(instance: String) -> Self {
        Self {
            instance,
            seq: 0,
            ring: VecDeque::new(),
            subscriptions: HashMap::new(),
            last: None,
        }
    }
    fn emit(&mut self, kind: &str, data: Value) {
        self.seq += 1;
        self.ring
            .push_back(json!({"cursor":self.seq,"instance":self.instance,"kind":kind,"data":data}));
        while self.ring.len() > 256 {
            self.ring.pop_front();
        }
    }
    pub fn observe(&mut self, snapshot: Snapshot) {
        if let Some(old) = self.last.replace(snapshot.clone()) {
            for id in old
                .tabs
                .keys()
                .filter(|id| !snapshot.tabs.contains_key(*id))
            {
                self.emit("tab", json!({"id":id,"change":"closed"}));
            }
            for (id, layout) in &snapshot.tabs {
                match old.tabs.get(id) {
                    None => self.emit("tab", json!({"id":id,"change":"opened"})),
                    Some(previous) if previous != layout => {
                        self.emit("layout", json!({"id":id,"change":"changed"}))
                    }
                    _ => {}
                }
            }
            if (old.focus, old.selected, old.active)
                != (
                    snapshot.focus.clone(),
                    snapshot.selected.clone(),
                    snapshot.active,
                )
            {
                self.emit("focus",json!({"view":snapshot.focus,"tab":snapshot.selected,"window_active":snapshot.active}));
            }
            for (id, state) in &snapshot.rdp {
                if old.rdp.get(id) != Some(state) {
                    self.emit(
                        "rdp",
                        json!({"id":id,"phase":state.0,"frame_available":state.1}),
                    );
                }
            }
            for id in old.rdp.keys().filter(|id| !snapshot.rdp.contains_key(*id)) {
                self.emit(
                    "rdp",
                    json!({"id":id,"phase":"removed","frame_available":false}),
                );
            }
        }
    }
    pub fn exchange(
        &mut self,
        method: &str,
        params: &Value,
        uid: u32,
        now: Instant,
    ) -> Result<Value> {
        self.subscriptions
            .retain(|_, s| now.duration_since(s.seen) < Duration::from_secs(60));
        let p = params.as_object().ok_or_else(invalid)?;
        if method == "workspace.events.subscribe" {
            if p.len() != 1 {
                return Err(invalid());
            }
            let kinds = p
                .get("types")
                .and_then(Value::as_array)
                .filter(|v| !v.is_empty() && v.len() <= 4)
                .ok_or_else(invalid)?;
            let mut types = HashSet::new();
            for kind in kinds {
                let kind = kind
                    .as_str()
                    .filter(|k| TYPES.contains(k))
                    .ok_or_else(invalid)?;
                if !types.insert(kind.to_owned()) {
                    return Err(invalid());
                }
            }
            if self.subscriptions.len() >= 64 {
                return Err(Error::new("LIMIT", "GUI event subscription limit"));
            }
            let id = ruwac_protocol::id("subscription");
            self.subscriptions.insert(
                id.clone(),
                Subscription {
                    uid,
                    types,
                    seen: now,
                },
            );
            return Ok(
                json!({"subscription":id,"instance":self.instance,"cursor":self.seq,"expires_after_idle_seconds":60,"capacity":256,"delivery":"poll","coalesced_observation_ms":100}),
            );
        }
        let id = p
            .get("subscription")
            .and_then(Value::as_str)
            .filter(|id| ruwac_protocol::valid_id(id, "subscription"))
            .ok_or_else(invalid)?;
        let s = self
            .subscriptions
            .get_mut(id)
            .filter(|s| s.uid == uid)
            .ok_or_else(|| {
                Error::new(
                    "STALE_SUBSCRIPTION",
                    "GUI event subscription is missing, expired or belongs to another actor",
                )
            })?;
        match method {
            "workspace.events.unsubscribe" if p.len() == 1 => {
                self.subscriptions.remove(id);
                Ok(json!({"unsubscribed":true}))
            }
            "workspace.events.poll" if p.len() == 3 => {
                let after = p
                    .get("after")
                    .and_then(Value::as_u64)
                    .filter(|c| *c <= self.seq)
                    .ok_or_else(invalid)?;
                let limit = p
                    .get("limit")
                    .and_then(Value::as_u64)
                    .filter(|n| (1..=50).contains(n))
                    .ok_or_else(invalid)? as usize;
                let oldest = self
                    .ring
                    .front()
                    .and_then(|v| v["cursor"].as_u64())
                    .unwrap_or(self.seq + 1);
                if after < oldest - 1 {
                    return Err(Error::new(
                        "EVENT_GAP",
                        "GUI event history was overwritten; query workspace.list and resubscribe",
                    ));
                }
                s.seen = now;
                let mut events = Vec::new();
                let mut cursor = after;
                for event in self
                    .ring
                    .iter()
                    .filter(|e| e["cursor"].as_u64().is_some_and(|c| c > after))
                {
                    if s.types.contains(event["kind"].as_str().unwrap_or("")) {
                        if events.len() == limit {
                            break;
                        }
                        events.push(event.clone());
                    }
                    cursor = event["cursor"].as_u64().unwrap();
                }
                Ok(
                    json!({"subscription":id,"instance":self.instance,"events":events,"cursor":cursor,"head":self.seq,"more":cursor<self.seq}),
                )
            }
            _ => Err(invalid()),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn subscribe(h: &mut Hub, uid: u32, now: Instant, types: Value) -> Value {
        h.exchange(
            "workspace.events.subscribe",
            &json!({"types":types}),
            uid,
            now,
        )
        .unwrap()
    }
    #[test]
    fn paging_filter_actor_and_expiry() {
        let now = Instant::now();
        let mut h = Hub::new("instance".into());
        let s = subscribe(&mut h, 7, now, json!(["focus"]));
        h.emit("tab", json!({"change":"opened"}));
        h.emit("focus", json!({"view":"one"}));
        h.emit("focus", json!({"view":"two"}));
        let q = json!({"subscription":s["subscription"],"after":0,"limit":1});
        assert_eq!(
            h.exchange("workspace.events.poll", &q, 8, now)
                .unwrap_err()
                .code,
            "STALE_SUBSCRIPTION"
        );
        let first = h.exchange("workspace.events.poll", &q, 7, now).unwrap();
        assert_eq!(first["events"].as_array().unwrap().len(), 1);
        assert_eq!(first["cursor"], 2);
        let q = json!({"subscription":s["subscription"],"after":2,"limit":1});
        assert_eq!(
            h.exchange("workspace.events.poll", &q, 7, now).unwrap()["cursor"],
            3
        );
        assert_eq!(
            h.exchange(
                "workspace.events.poll",
                &q,
                7,
                now + Duration::from_secs(61)
            )
            .unwrap_err()
            .code,
            "STALE_SUBSCRIPTION"
        );
    }
    #[test]
    fn overflow_is_explicit_and_filters_are_strict() {
        let now = Instant::now();
        let mut h = Hub::new("instance".into());
        let s = subscribe(&mut h, 7, now, json!(["tab"]));
        for _ in 0..257 {
            h.emit("tab", json!({"change":"opened"}));
        }
        assert_eq!(
            h.exchange(
                "workspace.events.poll",
                &json!({"subscription":s["subscription"],"after":0,"limit":50}),
                7,
                now
            )
            .unwrap_err()
            .code,
            "EVENT_GAP"
        );
        assert!(
            h.exchange(
                "workspace.events.subscribe",
                &json!({"types":["clipboard"]}),
                7,
                now
            )
            .is_err()
        );
        assert!(
            h.exchange(
                "workspace.events.subscribe",
                &json!({"types":["tab","tab"]}),
                7,
                now
            )
            .is_err()
        );
        assert_eq!(h.ring.len(), 256);
    }
    #[test]
    fn unsubscribe_and_instance_isolation() {
        let now = Instant::now();
        let mut h = Hub::new("instance".into());
        let s = subscribe(&mut h, 7, now, json!(["tab"]));
        let q = json!({"subscription":s["subscription"]});
        assert!(
            Hub::new("another".into())
                .exchange("workspace.events.unsubscribe", &q, 7, now)
                .is_err()
        );
        assert_eq!(
            h.exchange("workspace.events.unsubscribe", &q, 7, now)
                .unwrap()["unsubscribed"],
            true
        );
        assert!(
            h.exchange("workspace.events.unsubscribe", &q, 7, now)
                .is_err()
        );
    }
    #[test]
    fn subscription_capacity_recovers_after_expiry() {
        let now = Instant::now();
        let mut h = Hub::new("instance".into());
        for _ in 0..64 {
            subscribe(&mut h, 7, now, json!(["tab"]));
        }
        assert_eq!(
            h.exchange(
                "workspace.events.subscribe",
                &json!({"types":["tab"]}),
                7,
                now
            )
            .unwrap_err()
            .code,
            "LIMIT"
        );
        assert_eq!(h.subscriptions.len(), 64);
        subscribe(&mut h, 7, now + Duration::from_secs(61), json!(["tab"]));
        assert_eq!(h.subscriptions.len(), 1);
    }
}
