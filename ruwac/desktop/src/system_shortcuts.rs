//! With Map keys on (AltGr tap) and an RDP view focused, system shortcuts
//! (Alt+Tab, Super, …) go to the remote desktop; otherwise to this computer.
use gtk::{gdk, glib, prelude::*};
use std::{cell::Cell, rc::Rc};

pub struct Grab {
    window: glib::WeakRef<gtk::Window>,
    remote_focus: Box<dyn Fn(&gtk::Widget) -> bool>,
    inhibited: Cell<bool>,
    local: Cell<bool>,
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
            local: Cell::new(true),
        })
    }
    /// Re-evaluate after focus, activation or tab changes.
    pub fn update(&self) {
        let Some(window) = self.window.upgrade() else {
            return;
        };
        let local = self.local.get();
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
    pub fn set_local(&self, local: bool) {
        self.local.set(local);
        self.update();
    }
}
