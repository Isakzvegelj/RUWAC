//! Window-level shortcuts run before terminal/RDP input controllers.
use gtk::gdk;
use std::cell::Cell;

/// Ctrl+Shift+T creates a terminal; Ctrl+Shift+R chooses a saved RDP host.
pub fn new_tab(key: gdk::Key, mods: gdk::ModifierType) -> Option<bool> {
    if (mods - gdk::ModifierType::LOCK_MASK)
        != (gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK)
    {
        return None;
    }
    match key.to_unicode()?.to_ascii_lowercase() {
        't' => Some(false),
        'r' => Some(true),
        _ => None,
    }
}

pub fn altgr(key: gdk::Key) -> bool {
    matches!(key, gdk::Key::ISO_Level3_Shift | gdk::Key::Alt_R)
}
fn modifier(key: gdk::Key) -> bool {
    matches!(
        key,
        gdk::Key::Shift_L
            | gdk::Key::Shift_R
            | gdk::Key::Control_L
            | gdk::Key::Control_R
            | gdk::Key::Alt_L
            | gdk::Key::Super_L
            | gdk::Key::Super_R
            | gdk::Key::Caps_Lock
            | gdk::Key::Num_Lock
    )
}

/// AltGr tap toggles local keys: a clean press and release of AltGr with no
/// other key in between. AltGr held with a key stays layout input (@, \ …).
/// With local keys, system shortcuts such as Alt+Tab go to this computer's
/// desktop; otherwise an RDP view sends them to the remote desktop.
#[derive(Default)]
pub struct LocalMode {
    active: Cell<bool>,
    held: Cell<bool>,
    chorded: Cell<bool>,
}
impl LocalMode {
    pub fn active(&self) -> bool {
        self.active.get()
    }
    pub fn press(&self, key: gdk::Key) {
        if altgr(key) {
            self.held.set(true);
            self.chorded.set(false);
        } else if self.held.get() && !modifier(key) {
            self.chorded.set(true);
        }
    }
    /// Some(new state) when this release completes an AltGr tap.
    pub fn release(&self, key: gdk::Key) -> Option<bool> {
        if !altgr(key) || !self.held.replace(false) || self.chorded.get() {
            return None;
        }
        self.active.set(!self.active.get());
        Some(self.active.get())
    }
}

/// Explicit host chords leave Right Alt/AltGr available to the remote layout.
pub fn local_escape(key: gdk::Key, mods: gdk::ModifierType) -> bool {
    key == gdk::Key::Home
        && (mods - gdk::ModifierType::LOCK_MASK)
            == (gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::ALT_MASK)
}
pub fn fullscreen(key: gdk::Key, mods: gdk::ModifierType) -> bool {
    key == gdk::Key::Return
        && (mods - gdk::ModifierType::LOCK_MASK)
            == (gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::ALT_MASK)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_chords_do_not_steal_altgr() {
        let chord = gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::ALT_MASK;
        assert!(local_escape(gdk::Key::Home, chord));
        assert!(fullscreen(gdk::Key::Return, chord));
        for k in [
            gdk::Key::Alt_R,
            gdk::Key::ISO_Level3_Shift,
            gdk::Key::a,
            gdk::Key::F11,
        ] {
            assert!(!local_escape(k, chord) && !fullscreen(k, chord));
        }
    }
}

#[cfg(test)]
mod new_tab_tests {
    use super::*;
    #[test]
    fn new_tabs_require_exact_host_chords() {
        let chord = gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK;
        assert_eq!(new_tab(gdk::Key::T, chord), Some(false));
        assert_eq!(
            new_tab(gdk::Key::r, chord | gdk::ModifierType::LOCK_MASK),
            Some(true)
        );
        assert_eq!(new_tab(gdk::Key::t, gdk::ModifierType::CONTROL_MASK), None);
        assert_eq!(
            new_tab(gdk::Key::R, chord | gdk::ModifierType::ALT_MASK),
            None
        );
        assert_eq!(
            new_tab(gdk::Key::R, chord | gdk::ModifierType::SUPER_MASK),
            None
        );
    }
}

#[cfg(test)]
mod local_mode_tests {
    use super::*;
    #[test]
    fn altgr_tap_toggles_and_altgr_chords_do_not() {
        let mode = LocalMode::default();
        mode.press(gdk::Key::ISO_Level3_Shift);
        assert_eq!(mode.release(gdk::Key::ISO_Level3_Shift), Some(true));
        assert!(mode.active());
        // AltGr+v types @ on many layouts: no toggle.
        mode.press(gdk::Key::ISO_Level3_Shift);
        mode.press(gdk::Key::v);
        assert_eq!(mode.release(gdk::Key::ISO_Level3_Shift), None);
        assert!(mode.active());
        mode.press(gdk::Key::Alt_R);
        assert_eq!(mode.release(gdk::Key::Alt_R), Some(false));
    }
}
