use crate::config::{Action, Config};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Default)]
pub struct InputState {
    pressed: BTreeMap<(usize, u16), Action>,
    blocked: BTreeSet<(usize, u16)>,
    pub owned: [bool; 3],
}
impl InputState {
    pub fn held(&self, a: Action) -> bool {
        self.pressed.values().any(|v| *v == a)
    }
    pub fn transition(
        &mut self,
        device: usize,
        key: u16,
        down: bool,
        c: &Config,
    ) -> Option<(Action, bool)> {
        let id = (device, key);
        if !down {
            self.blocked.remove(&id);
            let a = self.pressed.remove(&id)?;
            return (!self.held(a)).then_some((a, false));
        }
        if !c.enabled {
            self.suppress(device, key, down);
            return None;
        }
        if self.blocked.contains(&id) || self.pressed.contains_key(&id) {
            return None;
        }
        let a = *c.bindings.iter().find(|(_, v)| **v == key)?.0;
        let was = self.held(a);
        self.pressed.insert(id, a);
        (!was).then_some((a, true))
    }
    pub fn cancel(&mut self) {
        self.blocked.extend(self.pressed.keys().copied());
        self.pressed.clear();
    }
    pub fn suppress(&mut self, device: usize, key: u16, down: bool) {
        self.pressed.remove(&(device, key));
        if down {
            self.blocked.insert((device, key));
        } else {
            self.blocked.remove(&(device, key));
        }
    }
    pub fn remove_device(&mut self, device: usize) {
        self.pressed.retain(|(d, _), _| *d != device);
        self.blocked.retain(|(d, _)| *d != device);
    }
    pub fn keys(&self) -> Vec<u16> {
        self.pressed
            .keys()
            .chain(self.blocked.iter())
            .map(|(_, k)| *k)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
    pub fn release_key(&mut self, key: u16) {
        self.pressed.retain(|(_, k), _| *k != key);
        self.blocked.retain(|(_, k)| *k != key);
    }
    pub fn active(&self) -> bool {
        !self.pressed.is_empty()
    }
    pub fn reconcile_buttons(&mut self) -> Vec<(usize, bool)> {
        let mut out = Vec::new();
        for (i, a) in [
            Action::LeftButton,
            Action::RightButton,
            Action::MiddleButton,
        ]
        .iter()
        .enumerate()
        {
            let wanted = self.held(*a);
            if wanted != self.owned[i] {
                out.push((i, wanted));
            }
        }
        out
    }
    pub fn commit_button(&mut self, i: usize, down: bool) {
        self.owned[i] = down;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transitions_and_devices() {
        let c = Config::default();
        let mut s = InputState::default();
        assert_eq!(s.transition(1, 0x7c, true, &c), Some((Action::Up, true)));
        assert!(s.transition(1, 0x7c, true, &c).is_none());
        assert!(s.transition(2, 0x7c, true, &c).is_none());
        s.remove_device(1);
        assert!(s.held(Action::Up));
        assert_eq!(s.transition(2, 0x7c, false, &c), Some((Action::Up, false)));
    }
    #[test]
    fn cancellation_requires_release() {
        let c = Config::default();
        let mut s = InputState::default();
        s.transition(1, 0x80, true, &c);
        assert_eq!(s.reconcile_buttons(), vec![(0, true)]);
        s.commit_button(0, true);
        s.cancel();
        assert_eq!(s.reconcile_buttons(), vec![(0, false)]);
        s.commit_button(0, false);
        assert!(s.transition(1, 0x80, true, &c).is_none());
        s.transition(1, 0x80, false, &c);
        assert!(s.transition(1, 0x80, true, &c).is_some());
    }
    #[test]
    fn disabled_and_lost_release() {
        let mut c = Config::default();
        let mut s = InputState::default();
        c.enabled = false;
        assert!(s.transition(1, 0x7c, true, &c).is_none());
        c.enabled = true;
        assert!(s.transition(1, 0x7c, true, &c).is_none());
        s.transition(1, 0x7c, false, &c);
        s.transition(1, 0x7c, true, &c);
        s.release_key(0x7c);
        assert!(!s.active());
    }
    #[test]
    fn grid_suppression_and_configuration_changes() {
        let mut c = Config::default();
        let mut s = InputState::default();
        s.suppress(1, 0x7c, true);
        assert!(s.transition(1, 0x7c, true, &c).is_none());
        s.suppress(1, 0x7c, false);
        assert!(s.transition(1, 0x7c, true, &c).is_some());
        s.cancel();
        c.bindings.insert(Action::Up, 0x7d);
        c.bindings.insert(Action::Down, 0x7c);
        assert!(s.transition(1, 0x7c, true, &c).is_none());
        s.transition(1, 0x7c, false, &c);
        assert_eq!(s.transition(1, 0x7c, true, &c), Some((Action::Down, true)));
    }
    #[test]
    fn randomized_transitions() {
        let c = Config::default();
        let mut s = InputState::default();
        let mut seed = 42u64;
        for _ in 0..20000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let k = 0x7c + (seed % 12) as u16;
            let d = ((seed >> 8) % 3) as usize;
            match (seed >> 16) % 8 {
                0 => s.cancel(),
                1 => s.remove_device(d),
                2 => s.release_key(k),
                _ => {
                    s.transition(d, k, seed & 0x100000 != 0, &c);
                }
            }
            for (i, down) in s.reconcile_buttons() {
                s.commit_button(i, down);
            }
            for (i, a) in [
                Action::LeftButton,
                Action::RightButton,
                Action::MiddleButton,
            ]
            .iter()
            .enumerate()
            {
                assert_eq!(s.owned[i], s.held(*a));
            }
        }
        s.cancel();
        for (i, d) in s.reconcile_buttons() {
            s.commit_button(i, d);
        }
        assert_eq!(s.owned, [false; 3]);
    }
}
