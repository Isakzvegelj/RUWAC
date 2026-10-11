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
