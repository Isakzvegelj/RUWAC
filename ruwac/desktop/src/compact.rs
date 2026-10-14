//! Centered, collapsible controls keep remote desktop corners unobstructed.
use gtk::{glib, prelude::*};
use std::cell::Cell;
use std::rc::Rc;

pub fn toolbar(parent: &gtk::Box, tools: &gtk::Box, name: &str, status: Option<&gtk::Label>) {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    let toggle = gtk::ToggleButton::builder()
        .icon_name("pan-down-symbolic")
        .build();
    toggle.set_widget_name(&format!("{name}-toggle"));
    toggle.set_tooltip_text(Some("Show controls (hover to preview; click to keep open)"));
    toggle.add_css_class("flat");
    if let Some(status) = status {
        status.set_hexpand(true);
        status.set_width_chars(1);
        status.set_max_width_chars(1);
        row.append(status);
        // Balance the status side so the disclosure stays at the window center.
        let balance = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        balance.set_hexpand(true);
        row.append(&toggle);
        row.append(&balance);
    } else {
        row.set_halign(gtk::Align::Center);
        row.append(&toggle);
    }
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .child(tools)
        .build();
    let reveal = gtk::Revealer::builder()
        .child(&scroll)
        .transition_duration(0)
        .build();
    reveal.set_widget_name(&format!("{name}-revealer"));
    parent.append(&row);
    parent.append(&reveal);
    let r = reveal.clone();
    toggle.connect_toggled(move |button| {
        r.set_reveal_child(button.is_active());
        button.set_icon_name(if button.is_active() {
            "pan-up-symbolic"
        } else {
            "pan-down-symbolic"
        });
    });
    let hovered = Rc::new(Cell::new(false));
    for widget in [row.upcast::<gtk::Widget>(), reveal.clone().upcast()] {
        let motion = gtk::EventControllerMotion::new();
        let (r, h) = (reveal.clone(), hovered.clone());
        motion.connect_enter(move |_, _, _| {
            h.set(true);
            r.set_reveal_child(true);
        });
        let (r, h, t) = (reveal.clone(), hovered.clone(), toggle.clone());
        motion.connect_leave(move |_| {
            h.set(false);
            let (r, h, t) = (r.clone(), h.clone(), t.clone());
            glib::timeout_add_local_once(std::time::Duration::from_millis(400), move || {
                if !h.get() && !t.is_active() {
                    r.set_reveal_child(false);
                }
            });
        });
        widget.add_controller(motion);
    }
}

pub fn fixture(window: &gtk::Window, q: &serde_json::Value) -> serde_json::Value {
    fn find(w: &gtk::Widget, name: &str) -> Option<gtk::Widget> {
        if w.widget_name() == name {
            return Some(w.clone());
        }
        let mut child = w.first_child();
        while let Some(c) = child {
            if let Some(found) = find(&c, name) {
                return Some(found);
            }
            child = c.next_sibling();
        }
        None
    }
    if let (Some(width), Some(height)) = (q["width"].as_i64(), q["height"].as_i64()) {
        window.set_default_size(
            width.clamp(500, 1600) as i32,
            height.clamp(500, 1200) as i32,
        );
    }
    let name = q["name"].as_str().unwrap_or("workspace-tools");
    if !["workspace-tools", "rdp-tools"].contains(&name) {
        return serde_json::json!({"error":"unknown toolbar"});
    }
    if let Some(open) = q["open"].as_bool() {
        if let Some(toggle) = find(window.upcast_ref(), &format!("{name}-toggle"))
            .and_then(|w| w.downcast::<gtk::ToggleButton>().ok())
        {
            toggle.set_active(open);
        }
    }
    let reveal = find(window.upcast_ref(), &format!("{name}-revealer"))
        .and_then(|w| w.downcast::<gtk::Revealer>().ok());
    serde_json::json!({"found":reveal.is_some(),"open":reveal.as_ref().is_some_and(|r|r.reveals_child()),"height":reveal.map(|r|r.height()),"width":window.width(),"window_height":window.height()})
}
