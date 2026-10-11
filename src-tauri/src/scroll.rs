use crate::{
    config::{Action, Scroll},
    input::InputState,
};
#[derive(Default)]
pub struct ScrollScheduler {
    next: [Option<f64>; 2],
}
#[derive(Debug, PartialEq, Eq)]
pub struct Wheel {
    pub horizontal: bool,
    pub amount: i32,
}
impl ScrollScheduler {
    pub fn reset(&mut self) {
        self.next = [None, None];
    }
    pub fn update(&mut self, input: &InputState, config: &Scroll, now: f64) -> Vec<Wheel> {
        let mut events = Vec::new();
        for (i, action) in [Action::ScrollPositive, Action::ScrollNegative]
            .iter()
            .enumerate()
        {
            if !input.held(*action) {
                self.next[i] = None;
                continue;
            }
            let first = self.next[i].is_none();
            if first || config.repeat && now >= self.next[i].unwrap_or(f64::MAX) {
                events.push(Wheel {
                    horizontal: input.held(Action::Horizontal),
                    amount: if i == 0 {
                        config.amount as i32
                    } else {
                        -(config.amount as i32)
                    },
                });
                // Missed repetitions are dropped; a delayed scheduler never bursts old wheel events.
                self.next[i] = Some(
                    now + if first {
                        config.delay_ms as f64 / 1000.
                    } else {
                        config.interval_ms as f64 / 1000.
                    },
                );
            }
        }
        events
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    #[test]
    fn delay_repeat_horizontal_and_release() {
        let c = Config::default();
        let mut input = InputState::default();
        let mut s = ScrollScheduler::default();
        input.transition(1, c.bindings[&Action::ScrollPositive], true, &c);
        assert_eq!(
            s.update(&input, &c.scroll, 0.),
            vec![Wheel {
                horizontal: false,
                amount: 120
            }]
        );
        assert!(s.update(&input, &c.scroll, 0.299).is_empty());
        assert_eq!(s.update(&input, &c.scroll, 0.300).len(), 1);
        input.transition(1, c.bindings[&Action::Horizontal], true, &c);
        assert_eq!(
            s.update(&input, &c.scroll, 0.371),
            vec![Wheel {
                horizontal: true,
                amount: 120
            }]
        );
        input.release_key(c.bindings[&Action::ScrollPositive]);
        assert!(s.update(&input, &c.scroll, 0.5).is_empty());
        input.transition(1, c.bindings[&Action::ScrollNegative], true, &c);
        assert_eq!(
            s.update(&input, &c.scroll, 0.5),
            vec![Wheel {
                horizontal: true,
                amount: -120
            }]
        );
    }
    #[test]
    fn no_repeat_and_no_catchup_burst() {
        let mut c = Config::default();
        c.scroll.repeat = false;
        let mut input = InputState::default();
        let mut s = ScrollScheduler::default();
        input.transition(1, c.bindings[&Action::ScrollPositive], true, &c);
        assert_eq!(s.update(&input, &c.scroll, 0.).len(), 1);
        assert!(s.update(&input, &c.scroll, 10.).is_empty());
        c.scroll.repeat = true;
        s.reset();
        assert_eq!(s.update(&input, &c.scroll, 10.).len(), 1);
        assert_eq!(s.update(&input, &c.scroll, 20.).len(), 1);
        assert!(s.update(&input, &c.scroll, 20.001).is_empty());
    }
}
