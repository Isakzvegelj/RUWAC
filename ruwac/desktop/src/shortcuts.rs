//! Window-level shortcuts run before terminal/RDP input controllers.
use gtk::gdk;

pub fn select_tab(tabs: &adw::TabView, key: gdk::Key, mods: gdk::ModifierType) -> bool {
    // Allow Caps/Num lock, but never claim Ctrl+Alt (AltGr), Shift or Super chords.
    let allowed = (gdk::ModifierType::ALT_MASK | gdk::ModifierType::LOCK_MASK).bits();
    if !mods.contains(gdk::ModifierType::ALT_MASK) || mods.bits() & !allowed != 0 {
        return false;
    }
    let index = match key.to_unicode() {
        Some('1'..='9') => key.to_unicode().unwrap() as i32 - '1' as i32,
        Some('0') => 9,
        _ => return false,
    };
    if index < tabs.n_pages() {
        tabs.set_selected_page(&tabs.nth_page(index));
    }
    // Missing tabs are deliberately a no-op; never type the chord into a remote session.
    true
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
