//! Native overlay/docked machine sidebar and private desktop preferences.
use adw::prelude::*;
use gtk::glib;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::PathBuf,
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    schema: u32,
    pinned: bool,
    width: u16,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            schema: 1,
            pinned: false,
            width: 260,
        }
    }
}
fn path() -> Result<PathBuf, String> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .ok_or("Private preferences unavailable")?
        .join("RUWAC");
    let m = fs::symlink_metadata(&base).map_err(|_| "Private preferences unavailable")?;
    if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o077 != 0 {
        return Err("Private preferences unavailable".into());
    }
    Ok(base.join("ui.json"))
}
fn load() -> Result<Settings, String> {
    let path = path()?;
    let file = match fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
    {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Settings::default()),
        Err(_) => return Err("Existing desktop preferences protected".into()),
    };
    let m = file.metadata().map_err(|_| "Preferences unavailable")?;
    if !m.is_file()
        || m.uid() != unsafe { libc::geteuid() }
        || m.mode() & 0o077 != 0
        || m.nlink() != 1
        || m.len() > 4096
    {
        return Err("Existing desktop preferences protected".into());
    }
    let mut bytes = Vec::new();
    file.take(4097)
        .read_to_end(&mut bytes)
        .map_err(|_| "Preferences unreadable")?;
    let value: Settings =
        serde_json::from_slice(&bytes).map_err(|_| "Existing desktop preferences protected")?;
    if value.schema != 1 || !(200..=420).contains(&value.width) {
        return Err("Existing desktop preferences protected".into());
    }
    Ok(value)
}
fn save(value: &Settings) -> Result<(), String> {
    load()?; // Refuse unknown or unsafe existing settings rather than replacing them.
    let target = path()?;
    let tmp = target.with_file_name(format!("{}.tmp", ruwac_protocol::id("ui")));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&tmp)
            .map_err(|_| "Preferences cannot be saved")?;
        file.write_all(&serde_json::to_vec(value).map_err(|_| "Invalid preferences")?)
            .map_err(|_| "Preferences cannot be saved")?;
        file.sync_all().map_err(|_| "Preferences cannot be saved")?;
        load()?;
        fs::rename(&tmp, &target).map_err(|_| "Preferences cannot be saved")?;
        fs::File::open(target.parent().unwrap())
            .and_then(|f| f.sync_all())
            .map_err(|_| "Preferences cannot be saved")?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(tmp);
    }
    result
}

pub struct Shell {
    pub view: adw::OverlaySplitView,
    pub button: gtk::ToggleButton,
    pub pinned: gtk::CheckButton,
    pub width: gtk::SpinButton,
}
impl Shell {
    pub fn new(
        sidebar: &gtk::Box,
        tabs: &adw::TabView,
        preferences: &gtk::Box,
        show: bool,
    ) -> Self {
        let loaded = load();
        let settings = loaded.clone().unwrap_or_default();
        let view = adw::OverlaySplitView::builder()
            .sidebar(sidebar)
            .content(tabs)
            .collapsed(!settings.pinned)
            .show_sidebar(show || settings.pinned)
            .min_sidebar_width(settings.width as f64)
            .max_sidebar_width(settings.width as f64)
            .sidebar_width_unit(adw::LengthUnit::Px)
            .vexpand(true)
            .hexpand(true)
            .build();
        let button = gtk::ToggleButton::with_label("Machines");
        button.set_widget_name("machines-toggle");
        button.set_tooltip_text(Some(
            "Show/hide Machines · Ctrl+K searches · Escape closes the overlay",
        ));
        button.set_active(view.shows_sidebar());
        let v = view.downgrade();
        button.connect_toggled(move |b| {
            if let Some(v) = v.upgrade() {
                v.set_show_sidebar(b.is_active());
            }
        });
        let b = button.downgrade();
        view.connect_show_sidebar_notify(move |v| {
            if let Some(b) = b.upgrade() {
                b.set_active(v.shows_sidebar());
            }
        });
        let pinned = gtk::CheckButton::with_label("Keep Machines sidebar pinned");
        pinned.set_widget_name("machines-pinned");
        pinned.set_active(settings.pinned);
        preferences.append(&pinned);
        preferences.append(&gtk::Label::new(Some("Machines sidebar width (pixels)")));
        let width = gtk::SpinButton::with_range(200.0, 420.0, 10.0);
        width.set_widget_name("machines-width");
        width.set_value(settings.width as f64);
        preferences.append(&width);
        let status = gtk::Label::new(None);
        status.set_wrap(true);
        preferences.append(&status);
        if loaded.is_err() {
            status.set_text("Preferences are unavailable; changes apply to this window");
        }
        let (v, w, s) = (view.downgrade(), width.downgrade(), status.clone());
        pinned.connect_toggled(move |p| {
            let (Some(v), Some(w)) = (v.upgrade(), w.upgrade()) else {
                return;
            };
            v.set_collapsed(!p.is_active());
            if p.is_active() {
                v.set_show_sidebar(true);
            }
            if save(&Settings {
                schema: 1,
                pinned: p.is_active(),
                width: w.value_as_int() as u16,
            })
            .is_err()
            {
                s.set_text("Preferences could not be saved; changes apply to this window");
            }
        });
        let (v, p, s) = (view.downgrade(), pinned.downgrade(), status);
        width.connect_value_changed(move |w| {
            let (Some(v), Some(p)) = (v.upgrade(), p.upgrade()) else {
                return;
            };
            v.set_min_sidebar_width(w.value());
            v.set_max_sidebar_width(w.value());
            if save(&Settings {
                schema: 1,
                pinned: p.is_active(),
                width: w.value_as_int() as u16,
            })
            .is_err()
            {
                s.set_text("Preferences could not be saved; changes apply to this window");
            }
        });
        let v = view.downgrade();
        tabs.connect_selected_page_notify(move |_| {
            let Some(v) = v.upgrade() else {
                return;
            };
            if v.is_collapsed() {
                v.set_show_sidebar(false);
            }
        });
        Self {
            view,
            button,
            pinned,
            width,
        }
    }
    pub fn fixture(&self, q: &serde_json::Value) -> serde_json::Value {
        if q["toggle"] == true {
            self.button.emit_clicked();
        }
        if let Some(pinned) = q["pinned"].as_bool() {
            self.pinned.set_active(pinned);
        }
        if let Some(width) = q["width"].as_f64() {
            if (200.0..=420.0).contains(&width) {
                self.width.set_value(width);
            }
        }
        if let Some(shown) = q["shown"].as_bool() {
            self.button.set_active(shown);
        }
        serde_json::json!({"shown":self.view.shows_sidebar(),"overlay":self.view.is_collapsed(),"pinned":self.pinned.is_active(),"width":self.width.value_as_int(),"allocated_sidebar_width":self.view.sidebar().map(|w|w.width()),"allocated_content_width":self.view.content().map(|w|w.width()),"allocated_width":self.view.width()})
    }
}
