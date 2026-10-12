//! Automatic GitHub checks and explicitly requested source installation off the GTK thread.
use adw::prelude::*;
use gtk::glib;
use serde_json::Value;
use std::{cell::Cell, rc::Rc, sync::mpsc, time::Duration};

fn request(args: &[&str]) -> Result<Value, String> {
    let output = std::process::Command::new("python3")
        .arg("-c")
        .arg(include_str!("../../../scripts/update-user.py"))
        .args(["--current", env!("CARGO_PKG_VERSION")])
        .args(args)
        .stderr(std::process::Stdio::null())
        .output()
        .map_err(|_| "Unable to start the update helper (Python 3 is required)".to_owned())?;
    let value: Value = serde_json::from_slice(&output.stdout)
        .map_err(|_| "Update helper returned an invalid response".to_owned())?;
    if output.status.success() {
        Ok(value)
    } else {
        Err(value["error"]
            .as_str()
            .unwrap_or("Update failed")
            .to_owned())
    }
}

fn task(args: &'static [&'static str], done: impl FnOnce(Result<Value, String>) + 'static) {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(request(args));
    });
    let mut done = Some(done);
    glib::timeout_add_local(Duration::from_millis(200), move || match rx.try_recv() {
        Ok(result) => {
            done.take().unwrap()(result);
            glib::ControlFlow::Break
        }
        Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
        Err(_) => {
            done.take().unwrap()(Err("Update worker stopped".into()));
            glib::ControlFlow::Break
        }
    });
}

pub fn attach(header: &adw::HeaderBar, window: &adw::ApplicationWindow, automatic: bool) {
    let button = gtk::Button::from_icon_name("software-update-available-symbolic");
    button.set_widget_name("ruwac-updates");
    button.set_tooltip_text(Some("Check GitHub for RUWAC updates"));
    header.pack_end(&button);
    let weak_window = window.downgrade();
    button.connect_clicked(move |_| {
        let Some(window) = weak_window.upgrade() else { return };
        let dialog = gtk::Window::builder().title("RUWAC updates").transient_for(&window)
            .modal(true).default_width(480).build();
        let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
        content.set_margin_top(18); content.set_margin_bottom(18); content.set_margin_start(18); content.set_margin_end(18);
        let status = gtk::Label::new(Some("Checking GitHub…"));
        status.set_wrap(true); status.set_widget_name("update-status");
        content.append(&status);
        content.append(&gtk::Label::new(Some("Fedora 44 · builds published source locally\nPrevious installation and running sessions are retained.")));
        let install = gtk::Button::with_label("Build and install update");
        install.set_widget_name("update-install"); install.set_sensitive(false);
        content.append(&install);
        let busy = Rc::new(Cell::new(false));
        let busy_install = busy.clone(); let label = status.clone();
        install.connect_clicked(move |button| {
            if busy_install.replace(true) { return; }
            button.set_sensitive(false);
            label.set_text("Building and installing… This may take several minutes. Existing windows keep running.");
            let label = label.downgrade(); let busy = busy_install.clone();
            task(&["--install"], move |result| {
                busy.set(false);
                if let Some(label) = label.upgrade() { label.set_text(&match result {
                    Ok(value) => format!("Installed {}. Close and reopen the frontend when ready; terminal sessions remain running.", value["installed"].as_str().unwrap_or(value["latest"].as_str().unwrap_or("current version"))),
                    Err(error) => format!("Update failed: {error}\nPrevious installation remains available. Use ruwac update --install in a terminal for build diagnostics."),
                }); }
            });
        });
        let rollback = gtk::Button::with_label("Restore previous installation");
        let label = status.clone(); let busy_rollback = busy.clone();
        rollback.connect_clicked(move |button| {
            if busy_rollback.replace(true) { return; }
            button.set_sensitive(false);
            let label = label.downgrade(); let busy = busy_rollback.clone();
            task(&["--rollback"], move |result| {
                busy.set(false);
                if let Some(label) = label.upgrade() { label.set_text(&match result {
                    Ok(value) => format!("Restored {}. Running windows and sessions remain unchanged.", value["installed"].as_str().unwrap_or("previous version")),
                    Err(error) => format!("Rollback refused: {error}"),
                }); }
            });
        });
        content.append(&rollback);
        let auto = gtk::CheckButton::with_label("Check GitHub automatically at startup and daily");
        auto.set_widget_name("update-auto-check"); auto.set_sensitive(false);
        content.append(&auto);
        let a = auto.downgrade();
        task(&["--auto-check", "status"], move |result| {
            if let (Some(a), Ok(value)) = (a.upgrade(), result) {
                a.set_active(value["automatic_check"] == true); a.set_sensitive(true);
                a.connect_toggled(|a| {
                    let weak = a.downgrade();
                    task(if a.is_active() { &["--auto-check", "on"] } else { &["--auto-check", "off"] }, move |result| {
                        if result.is_err() { if let Some(a) = weak.upgrade() { a.set_sensitive(false); a.set_tooltip_text(Some("Unable to save update preference; existing settings were preserved")); } }
                    });
                });
            }
        });
        dialog.set_child(Some(&content)); dialog.present();
        let label = status.downgrade(); let install = install.downgrade();
        task(&["--check"], move |result| { if let Some(label) = label.upgrade() { match result {
            Ok(value) => {
                let available = value["available"] == true;
                label.set_text(&format!("Running {} · Installed {} · GitHub {}{}", env!("CARGO_PKG_VERSION"), value["current"].as_str().unwrap_or("unknown"), value["latest"].as_str().unwrap_or("unknown"), if available { " — update available" } else { " — up to date" }));
                if let Some(install) = install.upgrade() { install.set_sensitive(available); }
            }
            Err(error) => label.set_text(&format!("Unable to check: {error}")),
        } } });
    });
    if automatic {
        let check_button = button.downgrade();
        let check = move || {
            let weak = check_button.clone();
            task(&["--check-auto"], move |result| {
                if let (Some(button), Ok(value)) = (weak.upgrade(), result) {
                    if value["available"] == true {
                        button.add_css_class("suggested-action");
                        button.set_tooltip_text(Some(&format!(
                            "RUWAC {} is available — check updates",
                            value["latest"].as_str().unwrap_or("update")
                        )));
                    } else {
                        button.remove_css_class("suggested-action");
                    }
                }
            });
        };
        check();
        glib::timeout_add_local(Duration::from_secs(86400), move || {
            check();
            glib::ControlFlow::Continue
        });
    }
}

/// Widget callbacks used only by the explicitly private desktop fixture dispatcher.
pub fn fixture(window: &adw::ApplicationWindow, command: &Value) -> Value {
    fn find(root: &gtk::Widget, name: &str) -> Option<gtk::Widget> {
        if root.widget_name() == name {
            return Some(root.clone());
        }
        let mut child = root.first_child();
        while let Some(widget) = child {
            if let Some(found) = find(&widget, name) {
                return Some(found);
            }
            child = widget.next_sibling();
        }
        None
    }
    if command["operation"] == "open" {
        if let Some(button) = find(window.upcast_ref(), "ruwac-updates")
            .and_then(|w| w.downcast::<gtk::Button>().ok())
        {
            button.emit_clicked();
        }
    }
    for widget in gtk::Window::list_toplevels() {
        if let Ok(dialog) = widget.downcast::<gtk::Window>() {
            if dialog.title().as_deref() != Some("RUWAC updates") {
                continue;
            }
            if let Some(name) = command["click"].as_str() {
                if let Some(button) =
                    find(dialog.upcast_ref(), name).and_then(|w| w.downcast::<gtk::Button>().ok())
                {
                    if button.is_sensitive() {
                        button.emit_clicked();
                    }
                }
            }
            if let Some(enabled) = command["automatic"].as_bool() {
                if let Some(button) = find(dialog.upcast_ref(), "update-auto-check")
                    .and_then(|w| w.downcast::<gtk::CheckButton>().ok())
                {
                    if button.is_sensitive() {
                        button.set_active(enabled);
                    }
                }
            }
            let status = find(dialog.upcast_ref(), "update-status")
                .and_then(|w| w.downcast::<gtk::Label>().ok())
                .map(|label| label.text().to_string());
            let install =
                find(dialog.upcast_ref(), "update-install").is_some_and(|w| w.is_sensitive());
            let automatic = find(dialog.upcast_ref(), "update-auto-check")
                .and_then(|w| w.downcast::<gtk::CheckButton>().ok())
                .map(|w| w.is_active());
            let result = serde_json::json!({"dialog":true,"status":status,"install_enabled":install,"automatic":automatic});
            if command["operation"] == "close" {
                dialog.close();
            }
            return result;
        }
    }
    serde_json::json!({"dialog":false})
}
