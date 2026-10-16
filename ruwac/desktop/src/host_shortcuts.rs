//! Saved host menus. No connection is made until an explicit action is clicked.
use adw::prelude::*;
use std::cell::RefCell;

thread_local! {
    static MENUS: RefCell<Option<(gtk::glib::WeakRef<adw::ApplicationWindow>, crate::profiles::ShortcutMenus)>> = const { RefCell::new(None) };
}

fn scroll(body: &gtk::Box) -> gtk::ScrolledWindow {
    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .min_content_width(290)
        .max_content_height(480)
        .propagate_natural_height(true)
        .child(body)
        .build()
}

pub fn install(
    window: &adw::ApplicationWindow,
    menus: crate::profiles::ShortcutMenus,
    sessions: std::rc::Rc<dyn Fn() -> gtk::Box>,
) -> gtk::MenuButton {
    MENUS.with(|state| *state.borrow_mut() = Some((window.downgrade(), menus.clone())));
    let button = gtk::MenuButton::new();
    button.set_widget_name("agent-host-menu");
    button.set_icon_name("applications-science-symbolic");
    button.set_tooltip_text(Some("Agents · saved hosts · Codex, Claude and Kimi"));
    button.update_property(&[gtk::accessible::Property::Label("Saved agent hosts")]);
    let pop = gtk::Popover::new();
    pop.connect_show(move |pop| {
        // Running sessions first, then hosts that can start new ones.
        let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
        body.append(&sessions());
        body.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        body.append(&menus(true));
        pop.set_child(Some(&scroll(&body)));
    });
    button.set_popover(Some(&pop));
    button
}

pub fn popup(area: &gtk::DrawingArea, x: f64, y: f64) {
    MENUS.with(|state| {
        let state = state.borrow();
        let Some((window, menus)) = state.as_ref() else {
            return;
        };
        let Some(window) = window.upgrade() else {
            return;
        };
        if area.root().as_ref() != Some(window.upcast_ref::<gtk::Root>()) {
            return;
        }
        let pop = gtk::Popover::new();
        pop.set_widget_name("terminal-host-menu");
        pop.set_parent(area);
        pop.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        pop.set_child(Some(&scroll(&menus(false))));
        pop.connect_closed(|pop| pop.unparent());
        pop.popup();
    });
}

/// Native fixture drives the actual menus without connecting to any saved host.
pub fn fixture(window: &adw::ApplicationWindow, q: &serde_json::Value) -> serde_json::Value {
    let widgets = RefCell::new(Vec::new());
    let sections = RefCell::new(Vec::new());
    crate::chrome::walk(window.upcast_ref(), &|w| {
        if w.widget_name() == "agent-host-menu" && q["operation"] == "header" {
            if let Some(b) = w.downcast_ref::<gtk::MenuButton>() {
                b.popup();
            }
        }
    });
    let clicked: RefCell<Option<gtk::Button>> = RefCell::new(None);
    for top in gtk::Window::list_toplevels() {
        if q["cancel_confirmation"] == true {
            if let Some(dialog) = top.downcast_ref::<gtk::Dialog>() {
                if matches!(
                    dialog.title().as_deref(),
                    Some("Install remote RUWAC CLI" | "Authorize SSH public key")
                ) {
                    dialog.response(gtk::ResponseType::Cancel);
                }
            }
        }
        crate::chrome::walk(&top, &|w| {
            if let Some(label) = w.downcast_ref::<gtk::Label>() {
                if [
                    "Recent SSH connections",
                    "Saved connections",
                    "Saved agent hosts",
                    "Running agent sessions",
                ]
                .contains(&label.text().as_str())
                {
                    sections.borrow_mut().push(label.text().to_string());
                }
            }
            if let Some(b) = w.downcast_ref::<gtk::Button>() {
                if clicked.borrow().is_none()
                    && q["click"].as_str() == Some(w.widget_name().as_str())
                {
                    *clicked.borrow_mut() = Some(b.clone());
                }
            }
            if w.widget_name().starts_with("saved-host-")
                || w.widget_name() == "agent-session"
                || w.widget_name().starts_with("machine-")
                || w.widget_name().starts_with("ssh-")
                || w.widget_name() == "terminal-host-menu"
            {
                widgets.borrow_mut().push(serde_json::json!({"name":w.widget_name().to_string(),"visible":w.is_visible(),"sensitive":w.is_sensitive(), "bounds":w.compute_bounds(window).map(|r| [r.x(),r.y(),r.width(),r.height()])}));
            }
        });
    }
    if let Some(button) = clicked.into_inner() {
        button.emit_clicked();
    }
    if q["close"] == true {
        crate::chrome::walk(window.upcast_ref(), &|w| {
            if let Some(p) = w.downcast_ref::<gtk::Popover>() {
                p.popdown();
            }
        });
    }
    serde_json::json!({"widgets":widgets.into_inner(), "sections":sections.into_inner()})
}
