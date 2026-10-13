//! A compositor can consume a Super chord after delivering its modifier press.
//! Defer that press until another remote key or an intentional focused tap.
use std::collections::BTreeSet;
#[derive(Default)]
pub struct DeferredSuper {
    pending: BTreeSet<u8>,
}
pub struct Plan {
    pub handled: bool,
    pub events: Vec<(u8, bool)>,
}
impl DeferredSuper {
    pub fn clear(&mut self) {
        self.pending.clear();
    }
    pub fn key(&mut self, super_key: Option<u8>, release: bool, focused: bool, held: bool) -> Plan {
        if let Some(scan) = super_key {
            if !release {
                self.pending.insert(scan);
                return Plan {
                    handled: true,
                    events: vec![],
                };
            }
            if self.pending.remove(&scan) {
                return Plan {
                    handled: true,
                    events: if focused {
                        vec![(scan, false), (scan, true)]
                    } else {
                        vec![]
                    },
                };
            }
            return Plan {
                handled: !held,
                events: vec![],
            };
        }
        let events = if !release {
            std::mem::take(&mut self.pending)
                .into_iter()
                .filter(|_| focused)
                .map(|scan| (scan, false))
                .collect()
        } else {
            vec![]
        };
        Plan {
            handled: false,
            events,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compositor_focus_switch_drops_unsent_modifier() {
        let mut m = DeferredSuper::default();
        assert!(m.key(Some(0x5b), false, true, false).events.is_empty());
        m.clear();
        let release = m.key(Some(0x5b), true, true, false);
        assert!(release.handled && release.events.is_empty());
    }
    #[test]
    fn focused_tap_and_delivered_chord_preserve_super() {
        let mut m = DeferredSuper::default();
        m.key(Some(0x5b), false, true, false);
        assert_eq!(
            m.key(Some(0x5b), true, true, false).events,
            vec![(0x5b, false), (0x5b, true)]
        );
        m.key(Some(0x5c), false, true, false);
        assert_eq!(m.key(None, false, true, false).events, vec![(0x5c, false)]);
        assert!(!m.key(Some(0x5c), true, true, true).handled);
    }
    #[test]
    fn inactive_release_does_not_send_a_tap() {
        let mut m = DeferredSuper::default();
        m.key(Some(0x5b), false, true, false);
        assert!(m.key(Some(0x5b), true, false, false).events.is_empty());
    }
}
