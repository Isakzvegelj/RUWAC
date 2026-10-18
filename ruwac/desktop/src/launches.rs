//! Explicit launch context survives frontend replacement; names never authorize it.
use crate::{
    launch_context::{Catalog, Entry, Recipe},
    profiles::{ImageProfileReader, Profile},
};
use gtk::glib;
use serde_json::Value;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    path::Path,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

type Reopen = Rc<dyn Fn(&str, &str, &str, &[String])>;
type Status = Rc<dyn Fn(&str, &str)>;
#[derive(Clone)]
struct Configuration {
    catalog: Option<Catalog>,
    root: std::path::PathBuf,
    daemon: String,
    reader: ImageProfileReader,
    status: Status,
}
thread_local! {
 static LAUNCHES:RefCell<HashMap<String,(String,Vec<String>)>>=RefCell::new(HashMap::new());
 static ENTRIES:RefCell<HashMap<String,Entry>>=RefCell::new(HashMap::new());
 static CONFIGURATION:RefCell<Option<Configuration>>=const{RefCell::new(None)};
 static READS:Cell<usize>=const{Cell::new(0)};
 static REOPEN:RefCell<Option<Reopen>>=const{RefCell::new(None)};
}
static WORKERS: AtomicUsize = AtomicUsize::new(0);
struct Permit;
impl Permit {
    fn acquire() -> Option<Self> {
        WORKERS
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < 8).then_some(n + 1)
            })
            .ok()
            .map(|_| Self)
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        WORKERS.fetch_sub(1, Ordering::AcqRel);
    }
}
fn status(session: &str, message: &str) {
    CONFIGURATION.with(|c| {
        if let Some(c) = c.borrow().as_ref() {
            (c.status)(session, message);
        }
    });
}
/// No remote connections or programs are opened while restoring metadata.
pub fn configure(
    root: &Path,
    daemon: String,
    reader: ImageProfileReader,
    status: Status,
) -> Result<(), String> {
    let catalog = Catalog::new(root, &daemon).ok();
    let available = catalog.is_some();
    CONFIGURATION.with(|c| {
        *c.borrow_mut() = Some(Configuration {
            catalog,
            root: root.into(),
            daemon,
            reader,
            status,
        })
    });
    refresh();
    if available {
        Ok(())
    } else {
        Err("Saved launch catalog unavailable; new sessions still work".into())
    }
}
/// Also called on opening Agents, so other frontend windows' launches appear.
pub fn refresh() {
    let Some(configuration) = CONFIGURATION.with(|c| c.borrow().clone()) else {
        return;
    };
    let daemon = configuration.daemon.clone();
    let Some(catalog) = configuration.catalog else {
        return;
    };
    let Some(permit) = Permit::acquire() else {
        return;
    };
    READS.with(|r| r.set(r.get() + 1));
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let _permit = permit;
        let _ = tx.send(catalog.list());
    });
    glib::timeout_add_local(Duration::from_millis(20), move || match rx.try_recv() {
        Ok(Ok(entries)) => {
            READS.with(|r| r.set(r.get() - 1));
            if !CONFIGURATION.with(|c| c.borrow().as_ref().is_some_and(|c| c.daemon == daemon)) {
                return glib::ControlFlow::Break;
            }
            ENTRIES.with(|s| {
                let mut s = s.borrow_mut();
                for entry in entries {
                    s.entry(entry.pane.clone()).or_insert(entry);
                }
            });
            glib::ControlFlow::Break
        }
        Ok(Err(_)) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            READS.with(|r| r.set(r.get() - 1));
            status(
                "",
                "Saved launch catalog could not be read; no endpoint was inferred",
            );
            glib::ControlFlow::Break
        }
        Err(_) => glib::ControlFlow::Continue,
    });
}
pub fn loading() -> bool {
    READS.with(|r| r.get() > 0)
}
/// Volatile compatibility for setup actions, never agent classification.
pub fn remember(session: &str, name: &str, argv: &[String]) {
    LAUNCHES.with(|l| {
        l.borrow_mut()
            .insert(session.into(), (name.into(), argv.to_vec()));
    });
}
fn persist(entry: Entry) {
    let Some(configuration) = CONFIGURATION.with(|c| c.borrow().clone()) else {
        return;
    };
    let Some(catalog) = configuration.catalog else {
        status(
            &entry.session,
            "Launch metadata unavailable; this connection remains usable for this frontend lifetime",
        );
        return;
    };
    let Some(permit) = Permit::acquire() else {
        status(
            &entry.session,
            "Launch metadata busy; this connection remains usable but is not saved yet",
        );
        return;
    };
    let session = entry.session.clone();
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let _permit = permit;
        let _ = tx.send(catalog.write(&entry));
    });
    glib::timeout_add_local(Duration::from_millis(20), move || match rx.try_recv() {
        Ok(Ok(())) => glib::ControlFlow::Break,
        Ok(Err(_)) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            status(
                &session,
                "Launch metadata could not be saved; this connection still works",
            );
            glib::ControlFlow::Break
        }
        Err(_) => glib::ControlFlow::Continue,
    });
}
pub fn remember_remote(
    reply: &Value,
    profile: &Profile,
    recipe: Recipe,
    name: &str,
    argv: &[String],
) {
    if let Some(session) = reply["session"].as_str() {
        remember(session, name, argv);
    }
    let Some(mut configuration) = CONFIGURATION.with(|c| c.borrow().clone()) else {
        return;
    };
    let Some(identity) = reply["_launch_daemon"].as_str() else {
        status(
            reply["session"].as_str().unwrap_or(""),
            "Daemon identity unavailable; launch endpoint was not inferred",
        );
        return;
    };
    if identity != configuration.daemon {
        ENTRIES.with(|e| e.borrow_mut().clear());
        LAUNCHES.with(|e| e.borrow_mut().clear());
        let _ = configure(
            &configuration.root,
            identity.into(),
            configuration.reader.clone(),
            configuration.status.clone(),
        );
        let Some(updated) = CONFIGURATION.with(|c| c.borrow().clone()) else {
            return;
        };
        configuration = updated;
    }
    let result = (|| {
        let session = reply["session"].as_str()?;
        let pane = reply["pane"].as_str()?;
        let pid = u32::try_from(reply["pid"].as_u64()?).ok()?;
        Entry::new(
            configuration.daemon,
            session,
            pane,
            reply["window"].as_str()?,
            pid,
            profile,
            recipe,
        )
        .ok()
    })();
    let Some(entry) = result else {
        status(
            reply["session"].as_str().unwrap_or(""),
            "Launch identity unavailable; no persistent endpoint association was inferred",
        );
        return;
    };
    remember(&entry.session, name, argv);
    ENTRIES.with(|e| {
        e.borrow_mut().insert(entry.pane.clone(), entry.clone());
    });
    persist(entry);
}
fn entry(session: &str, pane: &str) -> Option<Entry> {
    ENTRIES.with(|e| {
        e.borrow()
            .get(pane)
            .filter(|e| e.session == session)
            .cloned()
    })
}
fn profile(entry: &Entry) -> Option<Profile> {
    CONFIGURATION.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|c| (c.reader)(&entry.endpoint).ok())
            .filter(|p| entry.matches(p))
    })
}
/// Only the originally created pane/PID may use this SSH route, never a newly
/// split local shell in the same persistent session.
pub fn image_profile(session: &str, pane: &str, pid: u32) -> Option<Profile> {
    let e = entry(session, pane)?;
    if e.pid != pid {
        return None;
    }
    profile(&e)
}
pub fn is_remote(session: &str, pane: &str) -> bool {
    if entry(session, pane).is_some() {
        return true;
    }
    if ENTRIES.with(|e| e.borrow().values().any(|e| e.session == session)) {
        return false;
    }
    LAUNCHES.with(|l| l.borrow().contains_key(session))
}
pub fn is_agent(session: &str) -> bool {
    ENTRIES.with(|e| {
        e.borrow()
            .values()
            .any(|e| e.session == session && e.recipe.provider().is_some() && profile(e).is_some())
    })
}
pub fn agent_target(session: &str) -> Option<(String, String, u32)> {
    ENTRIES.with(|e| {
        e.borrow()
            .values()
            .find(|e| e.session == session && e.recipe.provider().is_some() && profile(e).is_some())
            .map(|e| (e.window.clone(), e.pane.clone(), e.pid))
    })
}
pub fn set_reopen(reopen: Reopen) {
    REOPEN.with(|r| *r.borrow_mut() = Some(reopen));
}
pub fn reopen(session: &str, pane: &str) -> bool {
    let planned = if let Some(e) = entry(session, pane) {
        let Some(p) = profile(&e) else {
            status(
                session,
                "Saved SSH endpoint changed or was removed; open the current bookmark explicitly",
            );
            return true;
        };
        let argv = match e.recipe.argv(&p) {
            Ok(a) => a,
            Err(_) => {
                status(
                    session,
                    "Saved SSH launch is unavailable; inspect its machine settings",
                );
                return true;
            }
        };
        Some((
            format!(
                "{}{}",
                p.name,
                e.recipe
                    .provider()
                    .map(|p| format!(" · {p}"))
                    .unwrap_or_default()
            ),
            argv,
        ))
    } else {
        if ENTRIES.with(|e| e.borrow().values().any(|e| e.session == session)) {
            return false;
        }
        LAUNCHES.with(|l| l.borrow().get(session).cloned())
    };
    let Some((name, argv)) = planned else {
        return false;
    };
    let Some(reopen) = REOPEN.with(|r| r.borrow().clone()) else {
        return false;
    };
    reopen(session, pane, &name, &argv);
    true
}
pub fn recreated(old: &str, pane: &str, reply: &Value, name: &str, argv: &[String]) {
    if let Some(e) = entry(old, pane) {
        if let Some(p) = profile(&e) {
            remember_remote(reply, &p, e.recipe, name, argv);
        }
    } else if let Some(id) = reply["session"].as_str() {
        remember(id, name, argv);
    }
}
/// Fixture metadata is IDs and kinds only; never commands or endpoint details.
pub fn inspect() -> Value {
    ENTRIES.with(|e|serde_json::json!({"entries":e.borrow().values().map(|e|serde_json::json!({"session":e.session,"pane":e.pane,"provider":e.recipe.provider()})).collect::<Vec<_>>()}))
}
