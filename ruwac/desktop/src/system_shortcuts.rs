//! While an RDP view has keyboard focus, system shortcuts (Alt+Tab, Super, …)
//! go to the remote desktop. Pressing AltGr hands the next shortcut back to
//! the local desktop for `shortcuts::LEADER_WINDOW`.
use gtk::{gdk, glib, prelude::*};
use std::{cell::Cell, rc::Rc, time::Instant};

pub struct Grab {
    window: glib::WeakRef<gtk::Window>,
    remote_focus: Box<dyn Fn(&gtk::Widget) -> bool>,
    inhibited: Cell<bool>,
    local_until: Cell<Option<Instant>>,
}
impl Grab {
    pub fn new(
        window: &impl IsA<gtk::Window>,
        remote_focus: impl Fn(&gtk::Widget) -> bool + 'static,
    ) -> Rc<Self> {
        Rc::new(Self {
            window: window.as_ref().downgrade(),
            remote_focus: Box::new(remote_focus),
            inhibited: Cell::new(false),
            local_until: Cell::new(None),
        })
    }
    /// Re-evaluate after focus, activation or tab changes.
    pub fn update(&self) {
        let Some(window) = self.window.upgrade() else {
            return;
        };
        let local = self
            .local_until
            .get()
            .is_some_and(|until| Instant::now() < until);
        let want = window.is_active()
            && !local
            && gtk::prelude::GtkWindowExt::focus(&window).is_some_and(|f| (self.remote_focus)(&f));
        if want == self.inhibited.get() {
            return;
        }
        let Some(toplevel) = window.surface().and_downcast::<gdk::Toplevel>() else {
            return;
        };
        if want {
            toplevel.inhibit_system_shortcuts(None::<&gdk::Event>);
        } else {
            toplevel.restore_system_shortcuts();
        }
        self.inhibited.set(want);
    }
    /// AltGr: the next shortcut belongs to the local desktop.
    pub fn hand_to_local(self: &Rc<Self>) {
        let window = crate::shortcuts::LEADER_WINDOW;
        self.local_until.set(Some(Instant::now() + window));
        self.update();
        let weak = Rc::downgrade(self);
        glib::timeout_add_local_once(window + std::time::Duration::from_millis(20), move || {
            if let Some(grab) = weak.upgrade() {
                grab.update();
            }
        });
    }
}
