//! Non-secret machine metadata and explicit, read-only Remmina preview.
use gtk::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    io::Read,
    path::{Path, PathBuf},
    rc::Rc,
};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub machine_id: String,
    #[serde(default)]
    pub preferences: Preferences,
    #[serde(default)]
    pub inherit: bool,
    pub name: String,
    pub group: String,
    pub protocol: String,
    pub server: String,
    pub username: String,
    pub secret_reference: String,
    pub favorite: bool,
    pub last_used: u64,
    pub tags: Vec<String>,
}
/// Nonsecret endpoint overrides. Empty optional values inherit application defaults.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Preferences {
    pub label: String,
    pub network: String,
    pub gateway: String,
    pub clipboard: u32,
    pub display: String,
    pub zoom: f64,
    pub fullscreen: bool,
    pub reconnect: bool,
    pub trust: String,
    pub identity_file: String,
    pub known_hosts: String,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            label: String::new(),
            network: String::new(),
            gateway: String::new(),
            clipboard: 0,
            display: "fit".into(),
            zoom: 1.0,
            fullscreen: false,
            reconnect: true,
            trust: String::new(),
            identity_file: String::new(),
            known_hosts: String::new(),
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Library {
    schema: u32,
    items: Vec<Profile>,
    #[serde(default)]
    groups: Vec<Group>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Group {
    id: String,
    path: String,
    defaults: Preferences,
}
impl Profile {
    fn identify(&mut self) {
        if self.id.is_empty() {
            self.id = ruwac_protocol::id("endpoint");
        }
        if self.machine_id.is_empty() {
            self.machine_id = ruwac_protocol::id("machine");
        }
        if self.secret_reference.is_empty() {
            self.secret_reference = self.id.clone();
        }
    }
}
pub fn endpoint(address: &str, default_port: u16) -> Result<(String, u16), String> {
    if address.is_empty()
        || address.len() > 1024
        || address.chars().any(|c| c.is_whitespace() || c.is_control())
        || address.starts_with('-')
    {
        return Err("Invalid endpoint".into());
    }
    if let Ok(socket) = address.parse::<std::net::SocketAddr>() {
        if socket.port() == 0 {
            return Err("Endpoint port cannot be zero".into());
        }
        return Ok((socket.ip().to_string(), socket.port()));
    }
    if address.parse::<std::net::Ipv6Addr>().is_ok() {
        return Ok((address.into(), default_port));
    }
    if let Some((host, port)) = address.rsplit_once(':') {
        let port = port.parse::<u16>().map_err(|_| "Invalid endpoint port")?;
        if host.is_empty() || host.contains(':') || port == 0 {
            return Err("Invalid endpoint".into());
        }
        return Ok((host.into(), port));
    }
    Ok((address.into(), default_port))
}
pub fn ssh_argv(p: &Profile) -> Result<Vec<String>, String> {
    let (host, port) = endpoint(&p.server, 22)?;
    let mut argv = vec![
        "ssh".into(),
        "-p".into(),
        port.to_string(),
        "-o".into(),
        "StrictHostKeyChecking=ask".into(),
    ];
    if !p.preferences.known_hosts.is_empty() {
        argv.extend([
            "-o".into(),
            format!("UserKnownHostsFile={}", p.preferences.known_hosts),
        ]);
    }
    if !p.preferences.identity_file.is_empty() {
        argv.extend([
            "-i".into(),
            p.preferences.identity_file.clone(),
            "-o".into(),
            "IdentitiesOnly=yes".into(),
        ]);
    }
    if !p.username.is_empty() {
        argv.extend(["-l".into(), p.username.clone()]);
    }
    argv.extend(["--".into(), host]);
    Ok(argv)
}
impl Profile {
    pub fn matches(&self, s: &str) -> bool {
        format!(
            "{} {} {} {}",
            self.name,
            self.group,
            self.server,
            self.tags.join(" ")
        )
        .to_lowercase()
        .contains(&s.to_lowercase())
    }
    fn validate(&self) -> Result<(), String> {
        if self.name.is_empty()
            || self.server.is_empty()
            || !matches!(self.protocol.as_str(), "RDP" | "SSH")
            || [
                &self.name,
                &self.group,
                &self.server,
                &self.username,
                &self.secret_reference,
            ]
            .iter()
            .any(|v| v.len() > 1024 || v.chars().any(char::is_control))
            || self.server.starts_with('-')
            || self.tags.len() > 64
        {
            return Err("Invalid profile metadata".into());
        }
        endpoint(&self.server, if self.protocol == "SSH" { 22 } else { 3389 })?;
        if self.preferences.clipboard > 4
            || !["fit", "native", "stretch", "dynamic"].contains(&self.preferences.display.as_str())
            || !self.preferences.zoom.is_finite()
            || !(0.25..=4.0).contains(&self.preferences.zoom)
            || self
                .tags
                .iter()
                .any(|s| s.len() > 1024 || s.chars().any(char::is_control))
            || [
                &self.preferences.label,
                &self.preferences.network,
                &self.preferences.gateway,
                &self.preferences.trust,
                &self.preferences.identity_file,
                &self.preferences.known_hosts,
                &self.id,
                &self.machine_id,
            ]
            .iter()
            .any(|s| s.len() > 1024 || s.chars().any(char::is_control))
        {
            return Err("Invalid endpoint preferences".into());
        }
        Ok(())
    }
}
/// This parser deliberately ignores credentials, certificates and key paths.
pub fn preview(path: &Path) -> Result<Profile, String> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| "Cannot open selected Remmina file")?;
    if !f
        .metadata()
        .map_err(|_| "Cannot inspect selected file")?
        .is_file()
    {
        return Err("Selected path is not a regular file".into());
    }
    let mut s = String::new();
    f.by_ref()
        .take(65537)
        .read_to_string(&mut s)
        .map_err(|_| "Invalid Remmina text")?;
    if s.len() > 65536 {
        return Err("Remmina file exceeds cap".into());
    }
    let mut fields = std::collections::HashMap::new();
    let mut selected = false;
    for line in s.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            selected = line == "[remmina]";
            continue;
        }
        if selected {
            if let Some((k, v)) = line.split_once('=') {
                if matches!(
                    k,
                    "name" | "group" | "protocol" | "server" | "username" | "labels"
                ) {
                    fields.insert(k, v);
                }
            }
        }
    }
    let field = |name| fields.get(name).copied().unwrap_or("").to_string();
    let p = Profile {
        id: String::new(),
        machine_id: String::new(),
        preferences: Preferences::default(),
        inherit: false,
        name: field("name"),
        group: field("group"),
        protocol: field("protocol"),
        server: field("server"),
        username: field("username"),
        tags: field("labels")
            .split(',')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect(),
        secret_reference: String::new(),
        favorite: false,
        last_used: 0,
    };
    p.validate()?;
    Ok(p)
}
#[derive(Default)]
struct Store {
    items: Vec<Profile>,
    path: Option<PathBuf>,
    error: Option<String>,
    groups: Vec<Group>,
    active: std::collections::HashSet<String>,
}
impl Store {
    fn load() -> Self {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .unwrap_or_default()
            .join("RUWAC");
        if !base.exists() {
            let _ = std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&base);
        }
        let Ok(m) = std::fs::symlink_metadata(&base) else {
            return Self::default();
        };
        if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o077 != 0 {
            return Self::default();
        }
        let path = base.join("machines.json");
        let mut items = Vec::new();
        let mut error = None;
        let mut groups = Vec::new();
        let mut migrate = false;
        match std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&path)
        {
            Ok(f) => {
                let valid = f.metadata().is_ok_and(|m| {
                    m.is_file()
                        && m.uid() == unsafe { libc::geteuid() }
                        && m.mode() & 0o077 == 0
                        && m.len() <= 1024 * 1024
                });
                let mut b = Vec::new();
                if valid && f.take(1024 * 1024 + 1).read_to_end(&mut b).is_ok() {
                    match parse_library(&b) {
                        Ok(v) => {
                            migrate = serde_json::from_slice::<serde_json::Value>(&b)
                                .is_ok_and(|v| v.is_array());
                            items = v;
                            groups = serde_json::from_slice::<Library>(&b)
                                .map(|l| l.groups)
                                .unwrap_or_default();
                        }
                        Err(e) => error = Some(e),
                    }
                } else {
                    error =
                        Some("Unsafe or unreadable metadata; existing library protected".into());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => {
                error = Some("Cannot open private metadata; existing library protected".into())
            }
        }
        let mut store = Self {
            items,
            path: Some(path.clone()),
            error,
            groups,
            active: Default::default(),
        };
        if store.error.is_none() && migrate {
            if let Err(e) = store.save() {
                store.error = Some(e);
            }
        }
        store
    }
    fn save(&self) -> Result<(), String> {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        if let Some(e) = &self.error {
            return Err(e.clone());
        }
        parse_library(
            &serde_json::to_vec(&Library {
                schema: 1,
                items: self.items.clone(),
                groups: self.groups.clone(),
            })
            .map_err(|_| "Invalid library")?,
        )?;
        let path = self
            .path
            .as_ref()
            .ok_or("Private profile storage unavailable")?;
        let tmp = path.with_file_name(format!("machines-{}.tmp", ruwac_protocol::id("save")));
        let result = (|| {
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(&tmp)
                .map_err(|e| e.to_string())?;
            let bytes = serde_json::to_vec_pretty(&Library {
                schema: 1,
                items: self.items.clone(),
                groups: self.groups.clone(),
            })
            .map_err(|e| e.to_string())?;
            if bytes.len() > 1024 * 1024 {
                return Err("Profile storage exceeds cap".into());
            }
            f.write_all(&bytes).map_err(|e| e.to_string())?;
            f.sync_all().map_err(|e| e.to_string())?;
            if path.exists() {
                let old = std::fs::OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                    .open(path)
                    .map_err(|_| "Cannot back up metadata")?;
                let mut bytes = Vec::new();
                old.take(1024 * 1024 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|_| "Cannot back up metadata")?;
                let backup = path.with_file_name("machines.previous.json");
                let backup_tmp = tmp.with_extension("backup");
                let mut out = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .custom_flags(libc::O_NOFOLLOW)
                    .open(&backup_tmp)
                    .map_err(|_| "Cannot create metadata backup")?;
                out.write_all(&bytes)
                    .map_err(|_| "Cannot write metadata backup")?;
                out.sync_all().map_err(|_| "Cannot sync metadata backup")?;
                std::fs::rename(&backup_tmp, backup)
                    .map_err(|_| "Cannot commit metadata backup")?;
            }
            std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
            std::fs::File::open(path.parent().unwrap())
                .and_then(|d| d.sync_all())
                .map_err(|_| "Cannot sync metadata directory".to_string())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(tmp);
        }
        result
    }
}
fn parse_library(b: &[u8]) -> Result<Vec<Profile>, String> {
    let value: serde_json::Value =
        serde_json::from_slice(b).map_err(|_| "Malformed metadata; existing library protected")?;
    let legacy = value.is_array();
    let mut items = if legacy {
        serde_json::from_value::<Vec<Profile>>(value).map_err(|_| "Invalid legacy metadata")?
    } else {
        let lib: Library = serde_json::from_value(value).map_err(|_| "Invalid metadata schema")?;
        if lib.groups.len() > 1000
            || lib.groups.iter().any(|g| {
                g.id.is_empty()
                    || g.path.is_empty()
                    || g.path.len() > 1024
                    || g.path.chars().any(char::is_control)
                    || g.defaults.clipboard > 4
                    || !["fit", "native", "stretch", "dynamic"]
                        .contains(&g.defaults.display.as_str())
                    || !g.defaults.zoom.is_finite()
                    || !(0.25..=4.0).contains(&g.defaults.zoom)
            })
        {
            return Err("Invalid network groups".into());
        }
        let mut ids = std::collections::HashSet::new();
        let mut paths = std::collections::HashSet::new();
        if lib
            .groups
            .iter()
            .any(|g| !ids.insert(g.id.clone()) || !paths.insert(g.path.clone()))
        {
            return Err("Conflicting network groups".into());
        }
        if lib.schema != 1 {
            return Err("Unsupported metadata schema; existing library protected".into());
        }
        lib.items
    };
    if items.len() > 1000 {
        return Err("Profile limit exceeded".into());
    }
    let mut ids = std::collections::HashSet::new();
    for p in &mut items {
        p.validate()?;
        if !legacy && (p.id.is_empty() || p.machine_id.is_empty()) {
            return Err("Missing stable endpoint identity".into());
        }
        p.identify();
        if !ids.insert(p.id.clone()) {
            return Err("Conflicting endpoint identities".into());
        }
    }
    Ok(items)
}
pub type PreferenceSink = Rc<dyn Fn(&str, f64, bool, u32)>;
pub fn sidebar(open: impl Fn(Profile, PreferenceSink) + 'static) -> (gtk::Box, gtk::SearchEntry) {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 6);
    root.set_width_request(240);
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("Machines, groups, tags (Ctrl+K)"));
    root.append(&search);
    let filter = gtk::DropDown::from_strings(&["All machines", "Favorites", "Recent"]);
    root.append(&filter);
    let status = gtk::Label::new(None);
    status.set_wrap(true);
    let list = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .child(&list)
        .build();
    root.append(&scroll);
    let store = Rc::new(RefCell::new(Store::load()));
    if let Some(e) = &store.borrow().error {
        status.set_text(e);
    }
    let callback: Rc<dyn Fn(Profile, PreferenceSink)> = Rc::new(open);
    let refresh: Rc<dyn Fn()> = {
        let list = list.clone();
        let store = store.clone();
        let search = search.clone();
        let filter = filter.clone();
        let status = status.clone();
        let callback = callback.clone();
        Rc::new(move || {
            while let Some(c) = list.first_child() {
                list.remove(&c);
            }
            let mut rows: Vec<_> = store
                .borrow()
                .items
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    p.matches(&search.text())
                        && match filter.selected() {
                            1 => p.favorite,
                            2 => p.last_used > 0,
                            _ => true,
                        }
                })
                .map(|(i, p)| (i, p.clone()))
                .collect();
            rows.sort_by_key(|(_, p)| {
                (
                    !p.favorite,
                    p.group.clone(),
                    std::cmp::Reverse(p.last_used),
                    p.name.clone(),
                )
            });
            for g in &store.borrow().groups {
                if !rows.iter().any(|(_, p)| p.group == g.path)
                    && (search.text().is_empty()
                        || g.path
                            .to_lowercase()
                            .contains(&search.text().to_lowercase()))
                {
                    list.append(&gtk::Label::new(Some(&g.path.replace('/', " › "))));
                }
            }
            let mut previous = String::new();
            for (i, p) in rows {
                if p.group != previous {
                    list.append(&gtk::Label::new(Some(&p.group.replace('/', " › "))));
                    previous = p.group.clone();
                }
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 2);
                let star = gtk::Button::with_label(if p.favorite { "★" } else { "☆" });
                let button = gtk::Button::with_label(&format!(
                    "{} · {} · {}",
                    p.name,
                    p.protocol,
                    if p.preferences.label.is_empty() {
                        &p.server
                    } else {
                        &p.preferences.label
                    }
                ));
                button.set_hexpand(true);
                row.append(&star);
                row.append(&button);
                let edit = gtk::Button::with_label("Edit");
                let (s, st) = (store.clone(), status.clone());
                let id = p.id.clone();
                edit.connect_clicked(move |b| editor(b, s.clone(), Some(id.clone()), st.clone()));
                row.append(&edit);
                list.append(&row);
                let s = store.clone();
                let label = star.clone();
                let st = status.clone();
                star.connect_clicked(move |_| {
                    let mut s = s.borrow_mut();
                    let id = s.items[i].machine_id.clone();
                    let favorite = !s.items[i].favorite;
                    for p in &mut s.items {
                        if p.machine_id == id {
                            p.favorite = favorite;
                        }
                    }
                    label.set_label(if s.items[i].favorite { "★" } else { "☆" });
                    if let Err(e) = s.save() {
                        st.set_text(&e);
                    }
                });
                let s = store.clone();
                let cb = callback.clone();
                let st = status.clone();
                button.connect_clicked(move |_| {
                    let store_handle = s.clone();
                    let mut s = s.borrow_mut();
                    s.items[i].last_used = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let mut p = s.items[i].clone();
                    if p.inherit {
                        if let Some(g) = s
                            .groups
                            .iter()
                            .filter(|g| {
                                p.group == g.path || p.group.starts_with(&(g.path.clone() + "/"))
                            })
                            .max_by_key(|g| g.path.len())
                        {
                            p.preferences.display = g.defaults.display.clone();
                            p.preferences.clipboard = g.defaults.clipboard;
                        }
                    }
                    s.active.insert(p.id.clone());
                    if let Err(e) = s.save() {
                        st.set_text(&e);
                    }
                    drop(s);
                    let (store, status, id) = (store_handle, st.clone(), p.id.clone());
                    let sink: PreferenceSink = Rc::new(move |mode, zoom, full, policy| {
                        let mut s = store.borrow_mut();
                        let old = s.items.clone();
                        if let Some(p) = s.items.iter_mut().find(|p| p.id == id) {
                            p.preferences.display = mode.into();
                            p.preferences.zoom = zoom;
                            p.preferences.fullscreen = full;
                            p.preferences.clipboard = policy;
                            p.inherit = false;
                        } else {
                            status.set_text("Endpoint removed; connection preferences not saved");
                            return;
                        }
                        if let Err(e) = s.save() {
                            s.items = old;
                            status.set_text(&e);
                        } else {
                            status.set_text("Connection preferences saved");
                        }
                    });
                    cb(p, sink);
                });
            }
        })
    };
    let groups = gtk::Button::with_label("Network groups / defaults…");
    root.append(&groups);
    let (s, st) = (store.clone(), status.clone());
    groups.connect_clicked(move |b| group_editor(b, s.clone(), st.clone()));
    let add = gtk::Button::with_label("New machine / endpoint…");
    root.append(&add);
    let (s, st) = (store.clone(), status.clone());
    add.connect_clicked(move |b| editor(b, s.clone(), None, st.clone()));
    let refresh_button = gtk::Button::with_label("Refresh machines");
    root.append(&refresh_button);
    let r = refresh.clone();
    refresh_button.connect_clicked(move |_| r());
    let undo = gtk::Button::with_label("Undo last metadata change");
    root.append(&undo);
    let (s, st, r) = (store.clone(), status.clone(), refresh.clone());
    undo.connect_clicked(move |_| {
        let mut s = s.borrow_mut();
        let result = (|| -> Result<(), String> {
            if !s.active.is_empty() {
                return Err(
                    "Close/restart workspace before restoring metadata used by active tabs".into(),
                );
            }
            let path = s
                .path
                .as_ref()
                .ok_or("Storage unavailable")?
                .with_file_name("machines.previous.json");
            use std::os::unix::fs::OpenOptionsExt;
            let f = std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(path)
                .map_err(|_| "No metadata backup")?;
            if !f
                .metadata()
                .is_ok_and(|m| m.is_file() && m.len() <= 1024 * 1024)
            {
                return Err("Unsafe backup".into());
            }
            let mut b = Vec::new();
            f.take(1024 * 1024 + 1)
                .read_to_end(&mut b)
                .map_err(|_| "Cannot read backup")?;
            let previous = s.items.clone();
            let prior_groups = s.groups.clone();
            let prior_error = s.error.clone();
            s.items = parse_library(&b)?;
            s.error = None;
            s.groups = serde_json::from_slice::<Library>(&b)
                .map(|l| l.groups)
                .unwrap_or_default();
            if let Err(e) = s.save() {
                s.items = previous;
                s.groups = prior_groups;
                s.error = prior_error;
                return Err(e);
            }
            Ok(())
        })();
        st.set_text(result.err().as_deref().unwrap_or("Metadata restored"));
        drop(s);
        r();
    });
    let r = refresh.clone();
    filter.connect_selected_notify(move |_| r());
    let r = refresh.clone();
    search.connect_changed(move |_| r());
    let selected = Rc::new(RefCell::new(None::<Profile>));
    let path = gtk::Entry::builder()
        .placeholder_text("Choose Remmina file explicitly")
        .build();
    root.append(&path);
    let preview_button = gtk::Button::with_label("Preview selected Remmina file");
    root.append(&preview_button);
    let conflict = gtk::DropDown::from_strings(&[
        "Skip duplicate endpoint",
        "Import duplicate as new identity",
    ]);
    root.append(&conflict);
    let import = gtk::Button::with_label("Import preview metadata");
    import.set_sensitive(false);
    root.append(&import);
    root.append(&status);
    let choose = gtk::Button::with_label("Choose file…");
    root.append(&choose);
    let entry = path.clone();
    choose.connect_clicked(move |b| {
        let dialog = gtk::FileDialog::new();
        let entry = entry.clone();
        let parent = b.root().and_then(|r| r.downcast::<gtk::Window>().ok());
        dialog.open(
            parent.as_ref(),
            None::<&gtk::gio::Cancellable>,
            move |result| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
                        entry.set_text(&path.to_string_lossy());
                    }
                }
            },
        );
    });
    let sel = selected.clone();
    let st = status.clone();
    let enable = import.clone();
    preview_button.connect_clicked(move |_| match preview(Path::new(path.text().as_str())) {
        Ok(p) => {
            st.set_text(&format!(
                "{} / {} · {} {}\nMetadata only; credentials excluded",
                p.group, p.name, p.protocol, p.server
            ));
            *sel.borrow_mut() = Some(p);
            enable.set_sensitive(true);
        }
        Err(e) => {
            st.set_text(&e);
            sel.borrow_mut().take();
            enable.set_sensitive(false);
        }
    });
    let st = status.clone();
    let r = refresh.clone();
    import.connect_clicked(move |_| {
        if let Some(mut p) = selected.borrow_mut().take() {
            p.identify();
            let mut s = store.borrow_mut();
            if s.items.len() >= 1000 {
                st.set_text("Profile limit reached");
                return;
            }
            if conflict.selected() == 1
                || !s.items.iter().any(|x| {
                    x.group == p.group
                        && x.name == p.name
                        && x.server == p.server
                        && x.protocol == p.protocol
                })
            {
                s.items.push(p);
            }
            match s.save() {
                Ok(()) => st.set_text("Machine metadata saved"),
                Err(e) => st.set_text(&e),
            }
            drop(s);
            r();
        }
    });
    refresh();
    (root, search)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seventy_nested_profiles_read_only() {
        let dir = std::env::temp_dir().join(ruwac_protocol::id("profiles"));
        std::fs::create_dir(&dir).unwrap();
        for i in 0..70 {
            let path = dir.join(format!("{i}.remmina"));
            let original = format!(
                "[remmina]\nname=Same leaf\ngroup=Team{}/Nested\nprotocol={}\nserver=127.0.0.1:{}\nusername=fixture\npassword=DO-NOT-IMPORT\nssh_privatekey=/private/key\nlabels=linux,test\n",
                i,
                if i % 2 == 0 { "RDP" } else { "SSH" },
                3000 + i
            );
            std::fs::write(&path, &original).unwrap();
            let p = preview(&path).unwrap();
            assert_eq!(p.group, format!("Team{i}/Nested"));
            assert!(p.matches("linux"));
            assert!(p.secret_reference.is_empty());
            assert!(!serde_json::to_string(&p).unwrap().contains("DO-NOT-IMPORT"));
            assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}

/// Exercises real sidebar widgets only within the desktop's explicit fixture mode.
pub fn fixture(
    root: &gtk::Box,
    file: Option<&Path>,
    query: Option<&str>,
    favorite: bool,
    open: bool,
) -> serde_json::Value {
    let mut child = root.first_child();
    while let Some(w) = child {
        child = w.next_sibling();
        if let Some(e) = w.downcast_ref::<gtk::SearchEntry>() {
            if let Some(q) = query {
                e.set_text(q);
            }
        }
        if let Some(e) = w.downcast_ref::<gtk::Entry>() {
            if let Some(path) = file {
                e.set_text(&path.to_string_lossy());
            }
        }
    }
    if file.is_some() {
        for label in ["Preview selected Remmina file", "Import preview metadata"] {
            let mut child = root.first_child();
            while let Some(w) = child {
                child = w.next_sibling();
                if let Some(b) = w.downcast_ref::<gtk::Button>() {
                    if b.label().as_deref() == Some(label) {
                        b.emit_clicked();
                    }
                }
            }
        }
    }
    fn collect(w: &gtk::Widget, out: &mut Vec<(gtk::Button, gtk::Button)>) {
        if let Some(row) = w.downcast_ref::<gtk::Box>() {
            if let (Some(a), Some(b)) = (
                row.first_child(),
                row.first_child().and_then(|w| w.next_sibling()),
            ) {
                if let (Ok(a), Ok(b)) = (a.downcast::<gtk::Button>(), b.downcast::<gtk::Button>()) {
                    if a.label().is_some_and(|l| l == "☆" || l == "★") {
                        out.push((a, b));
                    }
                }
            }
        }
        let mut child = w.first_child();
        while let Some(c) = child {
            child = c.next_sibling();
            collect(&c, out);
        }
    }
    let mut rows = Vec::new();
    collect(root.upcast_ref(), &mut rows);
    if open {
        if let Some((_, button)) = rows.first() {
            button.emit_clicked();
        }
    }
    if favorite {
        if let Some((star, _)) = rows.first() {
            star.emit_clicked();
        }
    }
    serde_json::json!({"rows":rows.len(),"favorites":rows.iter().filter(|(s,_)|s.label().as_deref()==Some("★")).count(),"labels":rows.iter().map(|(_,b)|b.label().unwrap_or_default().to_string()).collect::<Vec<_>>()})
}
#[cfg(test)]
mod special_file_tests {
    use super::*;
    #[test]
    fn fifo_preview_is_rejected_without_blocking() {
        use std::os::unix::ffi::OsStrExt;
        let root = std::env::temp_dir().join(ruwac_protocol::id("preview_fifo"));
        std::fs::create_dir(&root).unwrap();
        let fifo = root.join("special.remmina");
        let name = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let _ = tx.send(preview(&fifo));
        });
        assert!(
            rx.recv_timeout(std::time::Duration::from_secs(1))
                .unwrap()
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

fn editor(button: &gtk::Button, store: Rc<RefCell<Store>>, id: Option<String>, status: gtk::Label) {
    let existing = id
        .as_ref()
        .and_then(|id| store.borrow().items.iter().find(|p| &p.id == id).cloned());
    let mut original = existing.clone().unwrap_or(Profile {
        id: String::new(),
        machine_id: String::new(),
        preferences: Preferences::default(),
        inherit: false,
        name: String::new(),
        group: String::new(),
        protocol: "RDP".into(),
        server: String::new(),
        username: String::new(),
        secret_reference: String::new(),
        favorite: false,
        last_used: 0,
        tags: vec![],
    });
    original.identify();
    let parent = button.root().and_then(|w| w.downcast::<gtk::Window>().ok());
    let dialog = gtk::Dialog::builder()
        .title("Machine / connection properties")
        .modal(true)
        .default_width(560)
        .build();
    dialog.set_transient_for(parent.as_ref());
    dialog.add_button("Cancel", gtk::ResponseType::Cancel);
    dialog.add_button("Save", gtk::ResponseType::Accept);
    if existing.is_some() {
        dialog.add_button("Duplicate machine", gtk::ResponseType::Other(1));
        dialog.add_button("Add endpoint", gtk::ResponseType::Other(3));
        dialog.add_button("Delete endpoint…", gtk::ResponseType::Other(2));
        dialog.add_button("Delete machine…", gtk::ResponseType::Other(4));
    }
    let grid = gtk::Grid::builder()
        .row_spacing(5)
        .column_spacing(8)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(8)
        .margin_end(8)
        .build();
    let sc = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .min_content_height(440)
        .child(&grid)
        .build();
    dialog.content_area().append(&sc);
    let vals = [
        ("Machine name", original.name.clone()),
        ("Group / nested path", original.group.clone()),
        (
            "Machine identity (shared by endpoints)",
            original.machine_id.clone(),
        ),
        ("Address / port", original.server.clone()),
        ("Username", original.username.clone()),
        ("Endpoint label", original.preferences.label.clone()),
        ("Network", original.preferences.network.clone()),
        (
            "Gateway (metadata only)",
            original.preferences.gateway.clone(),
        ),
        ("Tags (comma separated)", original.tags.join(",")),
        ("Credential reference", original.secret_reference.clone()),
        (
            "RDP CA PEM trust reference",
            original.preferences.trust.clone(),
        ),
        (
            "SSH identity file",
            original.preferences.identity_file.clone(),
        ),
        (
            "SSH known_hosts file",
            original.preferences.known_hosts.clone(),
        ),
    ];
    let mut fields = Vec::new();
    for (i, (label, value)) in vals.iter().enumerate() {
        let e = gtk::Entry::builder().text(value).hexpand(true).build();
        grid.attach(&gtk::Label::new(Some(label)), 0, i as i32, 1, 1);
        grid.attach(&e, 1, i as i32, 1, 1);
        if i == 2 {
            e.set_editable(false);
        }
        fields.push(e);
    }
    let protocol = gtk::DropDown::from_strings(&["RDP", "SSH"]);
    protocol.set_selected(if original.protocol == "SSH" { 1 } else { 0 });
    let display = gtk::DropDown::from_strings(&["fit", "native", "stretch", "dynamic"]);
    display.set_selected(
        ["fit", "native", "stretch", "dynamic"]
            .iter()
            .position(|v| *v == original.preferences.display)
            .unwrap_or(0) as u32,
    );
    let clipboard = gtk::DropDown::from_strings(&[
        "Bidirectional",
        "Local → remote",
        "Remote → local",
        "Manual",
        "Disabled",
    ]);
    clipboard.set_selected(original.preferences.clipboard);
    for (i, (label, widget)) in [
        ("Protocol", &protocol),
        ("RDP display mode", &display),
        ("Clipboard policy", &clipboard),
    ]
    .iter()
    .enumerate()
    {
        grid.attach(&gtk::Label::new(Some(label)), 0, 13 + i as i32, 1, 1);
        grid.attach(*widget, 1, 13 + i as i32, 1, 1);
    }
    let zoom = gtk::SpinButton::with_range(0.25, 4.0, 0.25);
    zoom.set_value(original.preferences.zoom);
    grid.attach(&gtk::Label::new(Some("Zoom")), 0, 16, 1, 1);
    grid.attach(&zoom, 1, 16, 1, 1);
    let full = gtk::CheckButton::with_label("Fullscreen on connect");
    full.set_active(original.preferences.fullscreen);
    grid.attach(&full, 1, 17, 1, 1);
    let retry = gtk::CheckButton::with_label("Reconnect after transient transport loss");
    retry.set_active(original.preferences.reconnect);
    grid.attach(&retry, 1, 18, 1, 1);
    let inherit = gtk::CheckButton::with_label(
        "Inherit RDP defaults from nearest network group (saved overrides retained)",
    );
    inherit.set_active(original.inherit);
    grid.attach(&inherit, 0, 22, 2, 1);
    let secret = gtk::PasswordEntry::builder()
        .show_peek_icon(true)
        .placeholder_text("Set / replace credential in Secret Service")
        .build();
    grid.attach(&secret, 1, 19, 1, 1);
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    grid.attach(&actions, 1, 20, 1, 1);
    let info = gtk::Label::new(Some(
        "Endpoint ID is stable. Share a machine identity to add another endpoint. SSH verifies host keys; unknown keys prompt in the terminal. Gateway routing is not yet implemented.",
    ));
    info.set_wrap(true);
    grid.attach(&info, 0, 21, 2, 1);
    for (label, forget) in [
        ("Set / replace credential", false),
        ("Forget credential", true),
    ] {
        let b = gtk::Button::with_label(label);
        actions.append(&b);
        let (entry, reference, info) = (secret.clone(), fields[9].clone(), info.clone());
        b.connect_clicked(move |b| {
            let value = entry.text().to_string();
            entry.set_text("");
            let reference = reference.text().to_string();
            let (tx, rx) = std::sync::mpsc::sync_channel(1);
            b.set_sensitive(false);
            std::thread::spawn(move || {
                let _ = tx.send(if forget {
                    crate::credentials::forget(&reference)
                } else {
                    crate::credentials::set(&reference, value)
                });
            });
            let (b, info) = (b.clone(), info.clone());
            gtk::glib::timeout_add_local(std::time::Duration::from_millis(25), move || {
                match rx.try_recv() {
                    Ok(r) => {
                        b.set_sensitive(true);
                        info.set_text(
                            r.err()
                                .as_deref()
                                .unwrap_or("Credential operation completed in Secret Service"),
                        );
                        gtk::glib::ControlFlow::Break
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => gtk::glib::ControlFlow::Continue,
                    Err(_) => {
                        b.set_sensitive(true);
                        info.set_text("Secret Service worker unavailable");
                        gtk::glib::ControlFlow::Break
                    }
                }
            });
        });
    }
    dialog.connect_response(move|d,response|{
        if response==gtk::ResponseType::Other(2) || response==gtk::ResponseType::Other(4) {
            let machine=response==gtk::ResponseType::Other(4);if store.borrow().items.iter().any(|p|(p.id==original.id||(machine&&p.machine_id==original.machine_id))&&store.borrow().active.contains(&p.id)){info.set_text("This profile is used by an opened tab. Disconnect/close and restart before deleting it.");return;}
            let confirm=gtk::Dialog::builder().title("Delete endpoint metadata?").transient_for(d).modal(true).build();confirm.content_area().append(&gtk::Label::new(Some("Credentials remain in Secret Service. Undo restores metadata.")));confirm.add_button("Cancel",gtk::ResponseType::Cancel);confirm.add_button("Delete",gtk::ResponseType::Accept);
            let(s,id,machine_id,st,parent)=(store.clone(),original.id.clone(),original.machine_id.clone(),status.clone(),d.clone());confirm.connect_response(move|c,r|{if r==gtk::ResponseType::Accept {let mut s=s.borrow_mut();let old=s.items.clone();s.items.retain(|p|p.id!=id&&(!machine||p.machine_id!=machine_id));if let Err(e)=s.save(){s.items=old;st.set_text(&e);}else{drop(s);st.set_text("Metadata deleted; previous library retained for Undo");refresh_widgets(parent.transient_for().as_ref());parent.close();}}c.close();});confirm.present();return;
        }
        if response!=gtk::ResponseType::Accept && response!=gtk::ResponseType::Other(1) && response!=gtk::ResponseType::Other(3){d.close();return;}
        let mut p=original.clone();p.name=fields[0].text().into();p.group=fields[1].text().into();p.machine_id=fields[2].text().into();p.server=fields[3].text().into();p.username=fields[4].text().into();p.preferences.label=fields[5].text().into();p.preferences.network=fields[6].text().into();p.preferences.gateway=fields[7].text().into();p.tags=fields[8].text().split(',').map(str::trim).filter(|s|!s.is_empty()).map(str::to_owned).collect();p.secret_reference=fields[9].text().into();p.preferences.trust=fields[10].text().into();p.preferences.identity_file=fields[11].text().into();p.preferences.known_hosts=fields[12].text().into();p.protocol=if protocol.selected()==1{"SSH"}else{"RDP"}.into();p.preferences.display=["fit","native","stretch","dynamic"][display.selected() as usize].into();p.preferences.clipboard=clipboard.selected();p.preferences.zoom=zoom.value();p.preferences.fullscreen=full.is_active();p.preferences.reconnect=retry.is_active();p.inherit=inherit.is_active();
        if response==gtk::ResponseType::Other(1){p.id=ruwac_protocol::id("endpoint");p.machine_id=ruwac_protocol::id("machine");p.secret_reference=p.id.clone();p.name.push_str(" (copy)");}
        if response==gtk::ResponseType::Other(3){p.name=original.name.clone();p.group=original.group.clone();p.id=ruwac_protocol::id("endpoint");p.secret_reference=p.id.clone();}
        if let Err(e)=p.validate(){info.set_text(&e);return;}
        let mut s=store.borrow_mut();let old=s.items.clone();
        if let Some(i)=s.items.iter().position(|x|x.id==p.id){for item in &mut s.items{if item.machine_id==p.machine_id{item.name=p.name.clone();item.group=p.group.clone();item.tags=p.tags.clone();}}s.items[i]=p;}else{if s.items.len()>=1000{info.set_text("Profile limit reached");return;}let new_machine=p.machine_id.clone();s.items.push(p);
            if response==gtk::ResponseType::Other(1){for endpoint in old.iter().filter(|x|x.machine_id==original.machine_id&&x.id!=original.id){if s.items.len()>=1000{s.items=old.clone();info.set_text("Profile limit reached");return;}let mut copy=endpoint.clone();copy.id=ruwac_protocol::id("endpoint");copy.secret_reference=copy.id.clone();copy.machine_id=new_machine.clone();copy.name.push_str(" (copy)");s.items.push(copy);}}
        }
        if let Err(e)=s.save(){s.items=old;info.set_text(&e);return;}
        status.set_text("Machine metadata saved; existing tabs retain their connection settings");drop(s);refresh_widgets(parent.as_ref().map(|w|w.upcast_ref::<gtk::Widget>()));d.close();
    });
    dialog.present();
}
fn refresh_widgets(root: Option<&impl IsA<gtk::Widget>>) {
    fn visit(w: &gtk::Widget) {
        if let Some(b) = w.downcast_ref::<gtk::Button>() {
            if b.label().as_deref() == Some("Refresh machines") {
                b.emit_clicked();
                return;
            }
        }
        let mut c = w.first_child();
        while let Some(w) = c {
            c = w.next_sibling();
            visit(&w);
        }
    }
    if let Some(root) = root {
        visit(root.as_ref());
    }
}

fn group_editor(button: &gtk::Button, store: Rc<RefCell<Store>>, status: gtk::Label) {
    let parent = button.root().and_then(|w| w.downcast::<gtk::Window>().ok());
    let dialog = gtk::Dialog::builder()
        .title("Network groups and inherited defaults")
        .modal(true)
        .default_width(480)
        .build();
    dialog.set_transient_for(parent.as_ref());
    dialog.add_button("Cancel", gtk::ResponseType::Cancel);
    dialog.add_button("Save", gtk::ResponseType::Accept);
    dialog.add_button("Delete…", gtk::ResponseType::Other(2));
    dialog.add_button("Duplicate group", gtk::ResponseType::Other(3));
    let choose = gtk::DropDown::from_strings(
        &std::iter::once("New group".to_string())
            .chain(store.borrow().groups.iter().map(|g| g.path.clone()))
            .collect::<Vec<_>>()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    dialog.content_area().append(&choose);
    let path = gtk::Entry::builder()
        .placeholder_text("Full group path e.g. BASING/CIFRA/AHACIC")
        .build();
    dialog.content_area().append(&path);
    let mode = gtk::DropDown::from_strings(&["fit", "native", "stretch", "dynamic"]);
    dialog
        .content_area()
        .append(&gtk::Label::new(Some("Inherited RDP display mode")));
    dialog.content_area().append(&mode);
    let clip = gtk::DropDown::from_strings(&[
        "Bidirectional",
        "Local → remote",
        "Remote → local",
        "Manual",
        "Disabled",
    ]);
    dialog
        .content_area()
        .append(&gtk::Label::new(Some("Inherited clipboard policy")));
    dialog.content_area().append(&clip);
    let info = gtk::Label::new(Some(
        "Rename/move updates group paths and keeps machine, endpoint and group identities. Explicit endpoint overrides remain unchanged.",
    ));
    info.set_wrap(true);
    dialog.content_area().append(&info);
    let (s, p, m, c) = (store.clone(), path.clone(), mode.clone(), clip.clone());
    choose.connect_selected_notify(move |dropdown| {
        if let Some(g) = s
            .borrow()
            .groups
            .get(dropdown.selected().saturating_sub(1) as usize)
            .filter(|_| dropdown.selected() > 0)
        {
            p.set_text(&g.path);
            m.set_selected(
                ["fit", "native", "stretch", "dynamic"]
                    .iter()
                    .position(|v| *v == g.defaults.display)
                    .unwrap_or(0) as u32,
            );
            c.set_selected(g.defaults.clipboard);
        } else {
            p.set_text("");
        }
    });
    dialog.connect_response(move |d, r| {
        if r != gtk::ResponseType::Accept
            && r != gtk::ResponseType::Other(2)
            && r != gtk::ResponseType::Other(3)
        {
            d.close();
            return;
        }
        let path = path.text().trim_matches('/').to_string();
        if path.is_empty()
            || path.len() > 1024
            || path.chars().any(char::is_control)
            || path
                .split('/')
                .any(|p| p.is_empty() || p == "." || p == "..")
        {
            info.set_text("Enter a valid nonempty full group path");
            return;
        }
        let index = choose.selected();
        let mut s = store.borrow_mut();
        let old = s.groups.clone();
        let items = s.items.clone();
        if r == gtk::ResponseType::Other(2) {
            if index == 0 {
                info.set_text("Select a saved group");
                return;
            }
            let original = s.groups[(index - 1) as usize].path.clone();
            if s.items.iter().any(|p| {
                (p.group == original || p.group.starts_with(&(original.clone() + "/")))
                    && s.active.contains(&p.id)
            }) {
                info.set_text("Group is used by an opened tab; deletion protected");
                return;
            }
            drop(s);
            let confirm = gtk::Dialog::builder()
                .title("Delete group and endpoint metadata?")
                .transient_for(d)
                .modal(true)
                .build();
            confirm.add_button("Cancel", gtk::ResponseType::Cancel);
            confirm.add_button("Delete", gtk::ResponseType::Accept);
            let (s, st, parent) = (store.clone(), status.clone(), d.clone());
            confirm.connect_response(move |c, r| {
                if r == gtk::ResponseType::Accept {
                    let mut s = s.borrow_mut();
                    let (old, items) = (s.groups.clone(), s.items.clone());
                    s.groups.retain(|g| {
                        g.path != original && !g.path.starts_with(&(original.clone() + "/"))
                    });
                    s.items.retain(|p| {
                        p.group != original && !p.group.starts_with(&(original.clone() + "/"))
                    });
                    if let Err(e) = s.save() {
                        s.groups = old;
                        s.items = items;
                        st.set_text(&e);
                    } else {
                        drop(s);
                        st.set_text("Group deleted; metadata backup retained");
                        refresh_widgets(parent.transient_for().as_ref());
                        parent.close();
                    }
                }
                c.close();
            });
            confirm.present();
            return;
        }
        if s.groups
            .iter()
            .enumerate()
            .any(|(i, g)| g.path == path && i + 1 != index as usize)
        {
            info.set_text("That full group path already exists");
            return;
        }
        let defaults = Preferences {
            display: ["fit", "native", "stretch", "dynamic"][mode.selected() as usize].into(),
            clipboard: clip.selected(),
            ..Preferences::default()
        };
        if r == gtk::ResponseType::Other(3) && index > 0 {
            let original = s.groups[(index - 1) as usize].path.clone();
            if path == original {
                info.set_text("Enter a distinct full path for the duplicate group");
                return;
            }
            let mut machines = std::collections::HashMap::new();
            for group in &old {
                if group.path == original || group.path.starts_with(&(original.clone() + "/")) {
                    let mut g = group.clone();
                    g.id = ruwac_protocol::id("group");
                    g.path = format!("{}{}", path, &g.path[original.len()..]);
                    s.groups.push(g);
                }
            }
            for item in &items {
                if item.group == original || item.group.starts_with(&(original.clone() + "/")) {
                    let mut p = item.clone();
                    p.id = ruwac_protocol::id("endpoint");
                    p.secret_reference = p.id.clone();
                    p.machine_id = machines
                        .entry(p.machine_id.clone())
                        .or_insert_with(|| ruwac_protocol::id("machine"))
                        .clone();
                    p.group = format!("{}{}", path, &p.group[original.len()..]);
                    s.items.push(p);
                }
            }
        } else if index == 0 {
            s.groups.push(Group {
                id: ruwac_protocol::id("group"),
                path,
                defaults,
            });
        } else {
            let prior = s.groups[(index - 1) as usize].path.clone();
            for p in &mut s.items {
                if p.group == prior || p.group.starts_with(&(prior.clone() + "/")) {
                    p.group = format!("{}{}", path, &p.group[prior.len()..]);
                }
            }
            for g in &mut s.groups {
                if g.path == prior || g.path.starts_with(&(prior.clone() + "/")) {
                    g.path = format!("{}{}", path, &g.path[prior.len()..]);
                }
            }
            s.groups[(index - 1) as usize].defaults = defaults;
        }
        if let Err(e) = s.save() {
            s.groups = old;
            s.items = items;
            info.set_text(&e);
            return;
        }
        drop(s);
        status.set_text("Network groups saved; explicit endpoint preferences retained");
        refresh_widgets(parent.as_ref());
        d.close();
    });
    dialog.present();
}

/// Drives the same native dialogs as the owner, only in the explicitly private fixture mode.
pub fn fixture_editor(root: &gtk::Box, q: &serde_json::Value) -> serde_json::Value {
    fn find(w: &gtk::Widget, label: &str) -> Option<gtk::Button> {
        if let Some(b) = w.downcast_ref::<gtk::Button>() {
            if b.label().as_deref() == Some(label) {
                return Some(b.clone());
            }
        }
        let mut c = w.first_child();
        while let Some(w) = c {
            c = w.next_sibling();
            if let Some(b) = find(&w, label) {
                return Some(b);
            }
        }
        None
    }
    let action = q["operation"].as_str().unwrap_or("new");
    if action == "new" || action == "group" || action == "edit" {
        if let Some(b) = find(
            root.upcast_ref(),
            match action {
                "new" => "New machine / endpoint…",
                "group" => "Network groups / defaults…",
                _ => "Edit",
            },
        ) {
            b.emit_clicked();
        }
    }
    let title = if q["group"].as_bool() == Some(true) || action == "group" {
        "Network groups and inherited defaults"
    } else {
        "Machine / connection properties"
    };
    let mut applied = false;
    for w in gtk::Window::list_toplevels() {
        if let Ok(d) = w.downcast::<gtk::Dialog>() {
            if d.title().as_deref() != Some(title) {
                continue;
            }
            fn update(w: &gtk::Widget, q: &serde_json::Value) {
                if let Some(grid) = w.downcast_ref::<gtk::Grid>() {
                    let mut c = grid.first_child();
                    while let Some(child) = c {
                        c = child.next_sibling();
                        let (_, row, _, _) = grid.query_child(&child);
                        if let Some(e) = child.downcast_ref::<gtk::Entry>() {
                            if let Some(value) = q["fields"][row.to_string()].as_str() {
                                e.set_text(value);
                            }
                        }
                        if let Some(d) = child.downcast_ref::<gtk::DropDown>() {
                            if let Some(value) = q["fields"][row.to_string()].as_u64() {
                                d.set_selected(value as u32);
                            }
                        }
                    }
                }
                let mut c = w.first_child();
                while let Some(w) = c {
                    c = w.next_sibling();
                    update(&w, q);
                }
            }
            update(d.upcast_ref(), q);
            if title.starts_with("Network") {
                let mut child = d.content_area().first_child();
                let mut dropdown = 0;
                while let Some(w) = child {
                    child = w.next_sibling();
                    if let Some(e) = w.downcast_ref::<gtk::Entry>() {
                        if let Some(value) = q["path"].as_str() {
                            e.set_text(value);
                        }
                    }
                    if let Some(d) = w.downcast_ref::<gtk::DropDown>() {
                        if dropdown == 0 {
                            if let Some(index) = q["index"].as_u64() {
                                d.set_selected(index as u32);
                            }
                        }
                        dropdown += 1;
                    }
                }
            }
            let response = match q["response"].as_str().unwrap_or("save") {
                "cancel" => gtk::ResponseType::Cancel,
                "duplicate" => gtk::ResponseType::Other(1),
                "endpoint" => gtk::ResponseType::Other(3),
                "delete" => gtk::ResponseType::Other(2),
                "delete-machine" => gtk::ResponseType::Other(4),
                _ => gtk::ResponseType::Accept,
            };
            d.response(response);
            applied = true;
        }
    }
    if q["confirm"] == true {
        for w in gtk::Window::list_toplevels() {
            if let Ok(d) = w.downcast::<gtk::Dialog>() {
                if d.title().is_some_and(|s| s.starts_with("Delete ")) {
                    d.response(gtk::ResponseType::Accept);
                }
            }
        }
    }
    let s = Store::load();
    serde_json::json!({"applied":applied,"items":s.items,"groups":s.groups,"error":s.error})
}
#[cfg(test)]
mod workspace_tests {
    use super::*;
    #[test]
    fn migrate_thousand_preserve_identity_and_reject_corruption() {
        let original = preview_test();
        let legacy = serde_json::to_vec(&vec![original; 1000]).unwrap();
        let start = std::time::Instant::now();
        let mut items = parse_library(&legacy).unwrap();
        assert_eq!(items.len(), 1000);
        let ids: std::collections::HashSet<_> = items.iter().map(|p| p.id.clone()).collect();
        assert_eq!(ids.len(), 1000);
        let id = items[0].id.clone();
        let machine = items[0].machine_id.clone();
        items[0].name = "Łódź ž 🦀".into();
        items[0].group = "BASING/CIFRA/AHACIC".into();
        items[0].preferences.display = "native".into();
        items[0].preferences.zoom = 1.25;
        let bytes = serde_json::to_vec(&Library {
            schema: 1,
            items,
            groups: vec![],
        })
        .unwrap();
        assert!(bytes.len() < 1024 * 1024);
        let saved = parse_library(&bytes).unwrap();
        assert_eq!(saved[0].id, id);
        assert_eq!(saved[0].machine_id, machine);
        assert_eq!(saved[0].preferences.zoom, 1.25);
        assert!(parse_library(br#"{"schema":9,"items":[]}"#).is_err());
        assert!(parse_library(b"{broken").is_err());
        let mut duplicate = saved.clone();
        duplicate[1].id = duplicate[0].id.clone();
        assert!(parse_library(&serde_json::to_vec(&duplicate).unwrap()).is_err());
        println!(
            "1000-profile migration/search/serialization {} ms",
            start.elapsed().as_millis()
        );
        assert_eq!(saved.iter().filter(|p| p.matches("AHACIC")).count(), 1);
    }
    fn preview_test() -> Profile {
        Profile {
            id: String::new(),
            machine_id: String::new(),
            preferences: Preferences::default(),
            inherit: false,
            name: "server".into(),
            group: "network".into(),
            protocol: "RDP".into(),
            server: "localhost:3389".into(),
            username: "fixture".into(),
            secret_reference: String::new(),
            favorite: false,
            last_used: 0,
            tags: vec!["generated".into()],
        }
    }
    #[test]
    fn invalid_preferences_and_ports() {
        let mut p = preview_test();
        p.server = "localhost:0".into();
        assert!(p.validate().is_err());
        p.server = "localhost:22".into();
        p.preferences.zoom = f64::NAN;
        assert!(p.validate().is_err());
    }
}
