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
    let mut argv = vec!["ssh".into(), "-p".into(), port.to_string()];
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
        if let Ok(f) = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&path)
        {
            if f.metadata().is_ok_and(|m| {
                m.is_file()
                    && m.uid() == unsafe { libc::geteuid() }
                    && m.mode() & 0o077 == 0
                    && m.len() <= 1024 * 1024
            }) {
                let mut b = Vec::new();
                if f.take(1024 * 1024 + 1).read_to_end(&mut b).is_ok() {
                    if let Ok(v) = serde_json::from_slice::<Vec<Profile>>(&b) {
                        if v.len() <= 1000 && v.iter().all(|p| p.validate().is_ok()) {
                            items = v;
                        }
                    }
                }
            }
        }
        Self {
            items,
            path: Some(path),
        }
    }
    fn save(&self) -> Result<(), String> {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
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
            let bytes = serde_json::to_vec_pretty(&self.items).map_err(|e| e.to_string())?;
            if bytes.len() > 1024 * 1024 {
                return Err("Profile storage exceeds cap".into());
            }
            f.write_all(&bytes).map_err(|e| e.to_string())?;
            f.sync_all().map_err(|e| e.to_string())?;
            std::fs::rename(&tmp, path).map_err(|e| e.to_string())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(tmp);
        }
        result
    }
}
pub fn sidebar(open: impl Fn(Profile) + 'static) -> (gtk::Box, gtk::SearchEntry) {
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
    let callback: Rc<dyn Fn(Profile)> = Rc::new(open);
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
            let mut previous = String::new();
            for (i, p) in rows {
                if p.group != previous {
                    list.append(&gtk::Label::new(Some(&p.group.replace('/', " › "))));
                    previous = p.group.clone();
                }
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 2);
                let star = gtk::Button::with_label(if p.favorite { "★" } else { "☆" });
                let button = gtk::Button::with_label(&format!("{} · {}", p.name, p.protocol));
                button.set_hexpand(true);
                row.append(&star);
                row.append(&button);
                list.append(&row);
                let s = store.clone();
                let label = star.clone();
                let st = status.clone();
                star.connect_clicked(move |_| {
                    let mut s = s.borrow_mut();
                    s.items[i].favorite = !s.items[i].favorite;
                    label.set_label(if s.items[i].favorite { "★" } else { "☆" });
                    if let Err(e) = s.save() {
                        st.set_text(&e);
                    }
                });
                let s = store.clone();
                let cb = callback.clone();
                let st = status.clone();
                button.connect_clicked(move |_| {
                    let mut s = s.borrow_mut();
                    s.items[i].last_used = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let p = s.items[i].clone();
                    if let Err(e) = s.save() {
                        st.set_text(&e);
                    }
                    drop(s);
                    cb(p);
                });
            }
        })
    };
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
        if let Some(p) = selected.borrow_mut().take() {
            let mut s = store.borrow_mut();
            if s.items.len() >= 1000 {
                st.set_text("Profile limit reached");
                return;
            }
            if !s.items.iter().any(|x| {
                x.group == p.group
                    && x.name == p.name
                    && x.server == p.server
                    && x.protocol == p.protocol
            }) {
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
            if let (Some(a), Some(b)) = (row.first_child(), row.last_child()) {
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
