//! How SSH and agent tabs were started, so an ended connection can be reopened.
use std::{cell::RefCell, collections::HashMap, rc::Rc};

type Reopen = Rc<dyn Fn(&str, &str, &[String])>;

thread_local! {
    static LAUNCHES: RefCell<HashMap<String, (String, Vec<String>)>> = RefCell::new(HashMap::new());
    static REOPEN: RefCell<Option<Reopen>> = const { RefCell::new(None) };
}

/// Record the tab name and command a remote session was created with.
pub fn remember(session: &str, name: &str, argv: &[String]) {
    LAUNCHES.with(|l| {
        l.borrow_mut()
            .insert(session.into(), (name.into(), argv.to_vec()))
    });
}

/// Sessions started from a saved SSH endpoint or agent action.
pub fn is_remote(session: &str) -> bool {
    LAUNCHES.with(|l| l.borrow().contains_key(session))
}

/// The window installs how to open a session again (new tab, old one closed).
pub fn set_reopen(reopen: Reopen) {
    REOPEN.with(|r| *r.borrow_mut() = Some(reopen));
}

/// Open the same command again for an ended session; false when unknown.
pub fn reopen(session: &str) -> bool {
    let Some((name, argv)) = LAUNCHES.with(|l| l.borrow().get(session).cloned()) else {
        return false;
    };
    let Some(reopen) = REOPEN.with(|r| r.borrow().clone()) else {
        return false;
    };
    reopen(session, &name, &argv);
    true
}
