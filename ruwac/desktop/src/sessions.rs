use crate::{clipboard::Broker, view::View, worker::Rpc};
use adw::prelude::*;
use gtk::glib;
use serde_json::{Value, json};
use std::{cell::RefCell, path::PathBuf, rc::Rc, time::Duration};

pub fn append(
    tabs: &adw::TabView,
    views: &Rc<RefCell<Vec<Rc<View>>>>,
    socket: &PathBuf,
    broker: &Broker,
    id: &str,
    name: &str,
) {
    if let Some(v) = views.borrow().iter().find(|v| v.session == id).cloned() {
        if let Some(page) = crate::tiles::page(tabs, &v.root) {
            tabs.set_selected_page(&page);
        }
        v.focus();
        return;
    }
    let v = View::new(socket.clone(), id.into(), broker.clone());
    let page = crate::tiles::append(tabs, &v.root);
    page.set_title(name);
    views.borrow_mut().push(v.clone());
    tabs.set_selected_page(&page);
    v.focus();
}
pub fn controls(
    tabs: adw::TabView,
    views: Rc<RefCell<Vec<Rc<View>>>>,
    socket: PathBuf,
    broker: Broker,
) -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let add = gtk::Button::with_label("New terminal");
    let reopen = gtk::Button::with_label("Sessions…");
    root.append(&add);
    root.append(&reopen);
    let (t, v, s, b) = (tabs.clone(), views.clone(), socket.clone(), broker.clone());
    add.connect_clicked(move |button| {
        button.set_sensitive(false);
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let worker_socket = s.clone();
        std::thread::spawn(move || {
            let r = (|| {
                let mut rpc = Rpc::connect(&worker_socket)?;
                rpc.call("session.create", json!({"name":"Terminal"}))
            })();
            let _ = tx.send(r);
        });
        let (t, v, s, b, button) = (t.clone(), v.clone(), s.clone(), b.clone(), button.clone());
        glib::timeout_add_local(Duration::from_millis(20), move || match rx.try_recv() {
            Ok(r) => {
                button.set_sensitive(true);
                match r {
                    Ok(r) => {
                        if let Some(id) = r["session"].as_str() {
                            append(&t, &v, &s, &b, id, "Terminal");
                        }
                    }
                    Err(e) => button.set_tooltip_text(Some(&e.to_string())),
                }
                glib::ControlFlow::Break
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(_) => {
                button.set_sensitive(true);
                button.set_tooltip_text(Some("Session request failed"));
                glib::ControlFlow::Break
            }
        });
    });
    let (t, v, s, b) = (tabs, views, socket.clone(), broker);
    reopen.connect_clicked(move |button| {
        let dialog = gtk::Dialog::builder()
            .title("Persistent terminal sessions")
            .modal(true)
            .default_width(520)
            .build();
        if let Some(w) = button.root().and_then(|w| w.downcast::<gtk::Window>().ok()) {
            dialog.set_transient_for(Some(&w));
        }
        dialog.add_button("Close", gtk::ResponseType::Close);
        dialog.connect_response(|d, _| d.close());
        let status = gtk::Label::new(Some("Loading…"));
        dialog.content_area().append(&status);
        dialog.present();
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let socket = s.clone();
        std::thread::spawn(move || {
            let r = (|| {
                let mut rpc = Rpc::connect(&socket)?;
                rpc.call("session.list", json!({}))
            })();
            let _ = tx.send(r);
        });
        let (t, v, s, b, d) = (
            t.clone(),
            v.clone(),
            s.clone(),
            b.clone(),
            dialog.downgrade(),
        );
        glib::timeout_add_local(Duration::from_millis(20), move || {
            let Some(d) = d.upgrade().filter(|d| d.is_visible()) else {
                return glib::ControlFlow::Break;
            };
            match rx.try_recv() {
                Ok(r) => {
                    match r {
                        Ok(Value::Array(rows)) => {
                            status.set_text(
                                "Reopen a view. Closing a tab leaves its processes running.",
                            );
                            for row in rows {
                                if let (Some(id), Some(name)) =
                                    (row["id"].as_str(), row["name"].as_str())
                                {
                                    let button = gtk::Button::with_label(&format!(
                                        "{} · {} · {} windows",
                                        name,
                                        &id[..id.len().min(16)],
                                        row["windows"].as_array().map_or(0, Vec::len)
                                    ));
                                    button.set_tooltip_text(Some(id));
                                    d.content_area().append(&button);
                                    let (t, v, s, b, id, name, d) = (
                                        t.clone(),
                                        v.clone(),
                                        s.clone(),
                                        b.clone(),
                                        id.to_owned(),
                                        name.to_owned(),
                                        d.clone(),
                                    );
                                    button.connect_clicked(move |_| {
                                        append(&t, &v, &s, &b, &id, &name);
                                        d.close();
                                    });
                                }
                            }
                        }
                        Ok(_) => status.set_text("Invalid session response"),
                        Err(e) => status.set_text(&e.to_string()),
                    };
                    glib::ControlFlow::Break
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(_) => {
                    status.set_text("Session list failed");
                    glib::ControlFlow::Break
                }
            }
        });
    });
    root
}

pub fn fixture(widget: &impl IsA<gtk::Widget>, label: &str) -> bool {
    let widget = widget.as_ref();
    if let Some(button) = widget.downcast_ref::<gtk::Button>() {
        if button.label().as_deref() == Some(label)
            || button.tooltip_text().as_deref() == Some(label)
        {
            button.emit_clicked();
            return true;
        }
    }
    let mut child = widget.first_child();
    while let Some(w) = child {
        if fixture(&w, label) {
            return true;
        }
        child = w.next_sibling();
    }
    false
}
