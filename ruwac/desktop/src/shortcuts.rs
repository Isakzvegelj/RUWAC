//! Window-level shortcuts run before terminal/RDP input controllers.
use gtk::gdk;
use std::{
    cell::Cell,
    time::{Duration, Instant},
};

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

/// Workspace tab for 1…9 and 0 (tab 10); used after an AltGr tap.
pub fn tab_index(key: gdk::Key) -> Option<i32> {
    match key.to_unicode()? {
        c @ '1'..='9' => Some(c as i32 - '1' as i32),
        '0' => Some(9),
        _ => None,
    }
}
pub fn select_tab(tabs: &adw::TabView, index: i32) {
    // Missing tabs are deliberately a no-op.
    if index < tabs.n_pages() {
        tabs.set_selected_page(&tabs.nth_page(index));
    }
}

/// How long a released AltGr tap keeps the next key local.
pub const LEADER_WINDOW: Duration = Duration::from_millis(500);

fn altgr(key: gdk::Key) -> bool {
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

/// AltGr leader: a lone AltGr tap makes the next key, pressed within
/// LEADER_WINDOW, a local RUWAC shortcut. AltGr held together with another
/// key stays ordinary layout input for terminals and remote desktops, so
/// plain Alt+digit also reaches applications.
#[derive(Default)]
pub struct Leader {
    held: Cell<bool>,
    chorded: Cell<bool>,
    armed: Cell<Option<Instant>>,
}
impl Leader {
    /// True when this key press belongs to RUWAC rather than the application.
    pub fn press(&self, key: gdk::Key, now: Instant) -> bool {
        if altgr(key) {
            self.held.set(true);
            self.chorded.set(false);
            self.armed.set(None);
            return false;
        }
        if self.held.get() {
            self.chorded.set(true);
            return false;
        }
        if modifier(key) {
            return false;
        }
        self.armed.take().is_some_and(|deadline| now <= deadline)
    }
    pub fn release(&self, key: gdk::Key, now: Instant) {
        if altgr(key) && self.held.replace(false) && !self.chorded.get() {
            self.armed.set(Some(now + LEADER_WINDOW));
        }
    }
    pub fn armed(&self, now: Instant) -> bool {
        self.armed.get().is_some_and(|deadline| now <= deadline)
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
mod leader_tests {
    use super::*;
    #[test]
    fn altgr_tap_maps_only_the_next_key_within_half_a_second() {
        let (leader, t) = (Leader::default(), Instant::now());
        // Every tap re-arms.
        for round in 0..3u64 {
            let start = t + Duration::from_secs(round);
            assert!(!leader.press(gdk::Key::ISO_Level3_Shift, start));
            leader.release(gdk::Key::ISO_Level3_Shift, start);
            assert!(!leader.press(gdk::Key::Shift_L, start + Duration::from_millis(100)));
            assert!(leader.press(gdk::Key::_3, start + Duration::from_millis(400)));
            assert!(!leader.press(gdk::Key::_4, start + Duration::from_millis(450)));
        }
        // Too late.
        leader.press(gdk::Key::ISO_Level3_Shift, t);
        leader.release(gdk::Key::ISO_Level3_Shift, t);
        assert!(!leader.press(gdk::Key::_1, t + Duration::from_millis(501)));
        // AltGr held with a key is layout input (e.g. AltGr+2) and does not arm.
        leader.press(gdk::Key::Alt_R, t);
        assert!(!leader.press(gdk::Key::_2, t));
        leader.release(gdk::Key::Alt_R, t);
        assert!(!leader.press(gdk::Key::_1, t + Duration::from_millis(10)));
        assert_eq!(tab_index(gdk::Key::_1), Some(0));
        assert_eq!(tab_index(gdk::Key::_0), Some(9));
        assert_eq!(tab_index(gdk::Key::a), None);
    }
}
