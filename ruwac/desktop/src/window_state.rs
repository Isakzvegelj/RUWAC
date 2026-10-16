//! Private, per-window geometry. Workspace/session identities remain in workspace.rs.
use gtk::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    fs,
    io::{self, Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
    rc::Rc,
    time::Duration,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    schema: u32,
    width: i32,
    height: i32,
    maximized: bool,
    fullscreen: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            schema: 1,
            width: 1040,
            height: 780,
            maximized: false,
            fullscreen: false,
        }
    }
}
impl State {
    fn valid(&self) -> bool {
        self.schema == 1
            && (320..=16384).contains(&self.width)
            && (240..=16384).contains(&self.height)
    }
    fn sample(&mut self, window: &gtk::Window) {
        self.maximized = window.is_maximized();
        self.fullscreen = window.is_fullscreen();
        if !self.maximized && !self.fullscreen && window.width() >= 320 && window.height() >= 240 {
            self.width = window.width().min(16384);
            self.height = window.height().min(16384);
        }
    }
}

fn load(path: &Path) -> io::Result<State> {
    let mut file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let m = file.metadata()?;
    if !m.is_file()
        || m.uid() != unsafe { libc::geteuid() }
        || m.mode() & 0o777 != 0o600
        || m.len() > 4096
    {
        return Err(io::Error::other("unsafe window state"));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file).take(4097).read_to_end(&mut bytes)?;
    let state: State = serde_json::from_slice(&bytes)?;
    if !state.valid() {
        return Err(io::Error::other("invalid window state"));
    }
    Ok(state)
}
fn save(path: &Path, state: &State) -> io::Result<()> {
    if !state.valid() {
        return Err(io::Error::other("invalid window size"));
    }
    if fs::symlink_metadata(path).is_ok() {
        load(path)?;
    }
    let directory = path
        .parent()
        .ok_or_else(|| io::Error::other("window state directory unavailable"))?;
    let temporary = directory.join(ruwac_protocol::id("window-state"));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&temporary)?;
        file.write_all(&serde_json::to_vec(state)?)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        fs::File::open(directory)?.sync_all()
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

pub fn bind(window: &impl IsA<gtk::Window>) {
    let window = window.as_ref();
    let path = match crate::workspace::directory() {
        Ok(dir) => dir.join("window-state.json"),
        Err(_) => {
            eprintln!("Window geometry unavailable: unsafe configuration directory");
            return;
        }
    };
    let state = match load(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == io::ErrorKind::NotFound => State::default(),
        Err(_) => {
            eprintln!("Window geometry unavailable: saved state preserved for review");
            return;
        }
    };
    // Set the normal size before mapping, preventing an initial small viewport.
    window.set_default_size(state.width, state.height);
    if state.maximized {
        window.maximize();
    }
    if state.fullscreen {
        window.fullscreen();
    }
    let state = Rc::new(RefCell::new(state));
    let (closed_state, closed_path) = (state.clone(), path.clone());
    window.connect_close_request(move |window| {
        closed_state.borrow_mut().sample(window);
        if save(&closed_path, &closed_state.borrow()).is_err() {
            eprintln!("Window geometry save failed");
        }
        gtk::glib::Propagation::Proceed
    });
    let weak = window.downgrade();
    let mut previous = state.borrow().clone();
    gtk::glib::timeout_add_local(Duration::from_millis(750), move || {
        let Some(window) = weak.upgrade().filter(|w| w.is_visible()) else {
            return gtk::glib::ControlFlow::Break;
        };
        state.borrow_mut().sample(&window);
        if *state.borrow() != previous && save(&path, &state.borrow()).is_ok() {
            previous = state.borrow().clone();
        }
        gtk::glib::ControlFlow::Continue
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_or_future_geometry() {
        let mut state = State::default();
        assert!(state.valid());
        state.width = 0;
        assert!(!state.valid());
        state.width = 32768;
        assert!(!state.valid());
        state.width = 1040;
        state.schema = 2;
        assert!(!state.valid());
    }
    #[test]
    fn private_atomic_roundtrip_preserves_corruption_and_rejects_symlinks() {
        let dir = std::env::temp_dir().join(ruwac_protocol::id("geometry-test"));
        fs::create_dir(&dir).unwrap();
        let path = dir.join("state.json");
        let state = State {
            width: 1240,
            height: 880,
            maximized: true,
            ..State::default()
        };
        save(&path, &state).unwrap();
        assert_eq!(load(&path).unwrap(), state);
        assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o600);
        fs::write(&path, b"unrecognized generated metadata").unwrap();
        assert!(save(&path, &state).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"unrecognized generated metadata");
        let link = dir.join("link.json");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(load(&link).is_err());
        assert!(save(&link, &state).is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
