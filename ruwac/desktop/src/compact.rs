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
// Centered controls overlay content without changing its allocation.
use gtk::{glib, prelude::*};
use std::cell::Cell;
use std::rc::Rc;

pub fn toolbar(
    parent: &gtk::Box,
    tools: &gtk::Box,
    name: &str,
    status: Option<&gtk::Label>,
) -> gtk::Overlay {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    let toggle = gtk::ToggleButton::builder()
        .icon_name("pan-down-symbolic")
        .build();
    toggle.set_widget_name(&format!("{name}-toggle"));
    toggle.set_tooltip_text(Some("Show controls (hover to preview; click to keep open)"));
    toggle.add_css_class("flat");
    row.set_halign(gtk::Align::Center);
    row.append(&toggle);
    if let Some(status) = status {
        status.set_hexpand(true);
        status.set_width_chars(1);
        tools.prepend(status);
        let arrow = toggle.downgrade();
        status.connect_label_notify(move |label| {
            if let Some(arrow) = arrow.upgrade() {
                arrow.set_tooltip_text(Some(&format!("Show controls · {}", label.text())))
            }
        });
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
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    panel.set_widget_name(&format!("{name}-panel"));
    panel.set_valign(gtk::Align::Start);
    panel.set_halign(gtk::Align::Center);
    panel.add_css_class("background");
    panel.append(&row);
    panel.append(&reveal);
    let overlay = gtk::Overlay::new();
    overlay.set_hexpand(true);
    overlay.set_vexpand(true);
    overlay.add_overlay(&panel);
    overlay.set_measure_overlay(&panel, false);
    overlay.set_clip_overlay(&panel, true);
    parent.append(&overlay);
    if name == "rdp-tools" {
        // Stack both overlays without ever measuring them into the viewport.
        let cached = Rc::new(std::cell::RefCell::new(None::<glib::WeakRef<gtk::Widget>>));
        let cached_root = Rc::new(std::cell::RefCell::new(None::<glib::WeakRef<gtk::Window>>));
        panel.add_tick_callback(move |panel, _| {
            let current_root = panel.root().and_downcast::<gtk::Window>();
            let mut remembered_root = cached_root.borrow_mut();
            let mut cached = cached.borrow_mut();
            if remembered_root.as_ref().and_then(|r| r.upgrade()) != current_root {
                *cached = None;
                *remembered_root = current_root.map(|w| w.downgrade());
            }
            if cached.as_ref().and_then(|w| w.upgrade()).is_none() {
                if let Some(root) = panel.root().and_then(|r| {
                    r.downcast::<gtk::Window>()
                        .ok()
                        .map(|w| w.upcast::<gtk::Widget>())
                }) {
                    *cached = find(&root, "workspace-tools-panel").map(|w| w.downgrade());
                }
            }
            let offset = cached
                .as_ref()
                .and_then(|w| w.upgrade())
                .map_or(0, |w| w.height());
            if panel.margin_top() != offset {
                panel.set_margin_top(offset);
            }
            glib::ControlFlow::Continue
        });
    }
    let p = panel.downgrade();
    reveal.connect_child_revealed_notify(move |r| {
        if let Some(p) = p.upgrade() {
            p.set_halign(if r.reveals_child() {
                gtk::Align::Fill
            } else {
                gtk::Align::Center
            })
        }
    });
    let r = reveal.downgrade();
    toggle.connect_toggled(move |button| {
        if let Some(r) = r.upgrade() {
            r.set_reveal_child(button.is_active());
        }
        button.set_icon_name(if button.is_active() {
            "pan-up-symbolic"
        } else {
            "pan-down-symbolic"
        });
    });
    let hovered = Rc::new(Cell::new(false));
    for widget in [row.upcast::<gtk::Widget>(), reveal.clone().upcast()] {
        let motion = gtk::EventControllerMotion::new();
        let (r, h) = (reveal.downgrade(), hovered.clone());
        motion.connect_enter(move |_, _, _| {
            h.set(true);
            if let Some(r) = r.upgrade() {
                r.set_reveal_child(true);
            }
        });
        let (r, h, t) = (reveal.downgrade(), hovered.clone(), toggle.downgrade());
        motion.connect_leave(move |_| {
            h.set(false);
            let (r, h, t) = (r.clone(), h.clone(), t.clone());
            glib::timeout_add_local(std::time::Duration::from_millis(400), move || {
                let (Some(r), Some(t)) = (r.upgrade(), t.upgrade()) else {
                    return glib::ControlFlow::Break;
                };
                if h.get() || t.is_active() {
                    return glib::ControlFlow::Break;
                }
                let editing = r
                    .root()
                    .and_downcast::<gtk::Window>()
                    .and_then(|w| gtk::prelude::GtkWindowExt::focus(&w))
                    .is_some_and(|f| f.is_ancestor(&r));
                if editing {
                    return glib::ControlFlow::Continue;
                }
                r.set_reveal_child(false);
                glib::ControlFlow::Break
            });
        });
        widget.add_controller(motion);
    }
    overlay
}

pub fn fixture(window: &gtk::Window, q: &serde_json::Value) -> serde_json::Value {
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
    if let Some(hover) = q["hover"].as_bool() {
        if let Some(row) =
            find(window.upcast_ref(), &format!("{name}-toggle")).and_then(|w| w.parent())
        {
            let controllers = row.observe_controllers();
            for i in 0..controllers.n_items() {
                if let Some(motion) = controllers
                    .item(i)
                    .and_then(|c| c.downcast::<gtk::EventControllerMotion>().ok())
                {
                    if hover {
                        motion.emit_by_name::<()>("enter", &[&0.0f64, &0.0f64]);
                    } else {
                        motion.emit_by_name::<()>("leave", &[]);
                    }
                }
            }
        }
    }
    let reveal = find(window.upcast_ref(), &format!("{name}-revealer"))
        .and_then(|w| w.downcast::<gtk::Revealer>().ok());
    let viewport = find(window.upcast_ref(), &format!("{name}-panel"))
        .and_then(|p| p.parent())
        .and_then(|w| w.downcast::<gtk::Overlay>().ok())
        .and_then(|o| o.child())
        .map(|w| [w.width(), w.height()]);
    serde_json::json!({"viewport":viewport,"found":reveal.is_some(),"open":reveal.as_ref().is_some_and(|r|r.reveals_child()),"height":reveal.map(|r|r.height()),"width":window.width(),"window_height":window.height()})
}
