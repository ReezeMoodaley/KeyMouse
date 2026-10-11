use crate::{
    config::{Action, Config, Curve},
    input::InputState,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Normal,
    Precision,
    Turbo,
}
#[derive(Default)]
pub struct Movement {
    pub mode: Mode,
    direction: (i32, i32),
    started: f64,
    last: f64,
    fraction: (f64, f64),
}
pub fn speed(c: &Curve, elapsed: f64) -> f64 {
    c.min_speed
        + (c.max_speed - c.min_speed)
            * (elapsed / (c.ramp_ms / 1000.))
                .clamp(0., 1.)
                .powf(c.exponent)
}
pub fn direction(s: &InputState) -> (i32, i32) {
    (
        s.held(Action::Right) as i32 - s.held(Action::Left) as i32,
        s.held(Action::Down) as i32 - s.held(Action::Up) as i32,
    )
}
pub fn mode(s: &InputState) -> Mode {
    if s.held(Action::Precision) {
        Mode::Precision
    } else if s.held(Action::Turbo) {
        Mode::Turbo
    } else {
        Mode::Normal
    }
}
impl Movement {
    pub fn reset(&mut self, now: f64) {
        *self = Self {
            last: now,
            started: now,
            ..Self::default()
        };
    }
    pub fn input(
        &mut self,
        s: &InputState,
        c: &Config,
        now: f64,
        new_direction: Option<Action>,
    ) -> (i32, i32) {
        let d = direction(s);
        let m = mode(s);
        let mode_change = m != self.mode;
        if d != self.direction || mode_change {
            // Keep ramp for same-sign axis additions; reset on reversal, start, stop or mode change.
            if self.direction == (0, 0)
                || d == (0, 0)
                || mode_change
                || d.0 * self.direction.0 < 0
                || d.1 * self.direction.1 < 0
            {
                self.started = now;
            }
            if d.0 != self.direction.0 || mode_change {
                self.fraction.0 = 0.;
            }
            if d.1 != self.direction.1 || mode_change {
                self.fraction.1 = 0.;
            }
            self.direction = d;
            self.mode = m;
            self.last = now;
        }
        if m == Mode::Precision && !mode_change {
            if let Some(a) = new_direction {
                self.started = now;
                self.last = now;
                let n = c.precision.step as i32;
                return match a {
                    Action::Left if d.0 < 0 => (-n, 0),
                    Action::Right if d.0 > 0 => (n, 0),
                    Action::Up if d.1 < 0 => (0, -n),
                    Action::Down if d.1 > 0 => (0, n),
                    _ => (0, 0),
                };
            }
        }
        (0, 0)
    }
    pub fn tick(&mut self, c: &Config, now: f64) -> Result<(i32, i32), ()> {
        let dt = (now - self.last).max(0.);
        self.last = now;
        if dt > 0.100 {
            self.reset(now);
            return Err(());
        }
        if self.direction == (0, 0) {
            self.fraction = (0., 0.);
            return Ok((0, 0));
        }
        let (curve, delay) = match self.mode {
            Mode::Normal => (&c.normal, 0.),
            Mode::Turbo => (&c.turbo, 0.),
            Mode::Precision => (&c.precision.curve, c.precision.hold_ms / 1000.),
        };
        let hold_start = self.started + delay;
        if now <= hold_start {
            return Ok((0, 0));
        }
        let active_dt = dt.min((now - hold_start).max(0.));
        let length = ((self.direction.0 * self.direction.0 + self.direction.1 * self.direction.1)
            as f64)
            .sqrt();
        let v = speed(curve, now - hold_start) * active_dt;
        self.fraction.0 += self.direction.0 as f64 / length * v;
        self.fraction.1 += self.direction.1 as f64 / length * v;
        let out = (
            self.fraction.0.trunc() as i32,
            self.fraction.1.trunc() as i32,
        );
        self.fraction.0 -= out.0 as f64;
        self.fraction.1 -= out.1 as f64;
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn press(s: &mut InputState, c: &Config, a: Action) {
        s.transition(1, c.bindings[&a], true, c);
    }
    #[test]
    fn curve_limits() {
        let c = Config::default();
        assert_eq!(speed(&c.normal, 0.), 40.);
        assert_eq!(speed(&c.normal, 9.), 1400.);
        assert!((speed(&c.normal, 0.45) - (40. + 1360. * 0.5_f64.powf(1.6))).abs() < 1e-9);
    }
    #[test]
    fn diagonals_and_opposites() {
        let c = Config::default();
        let mut s = InputState::default();
        press(&mut s, &c, Action::Up);
        press(&mut s, &c, Action::Right);
        let mut m = Movement::default();
        m.input(&s, &c, 0., None);
        let mut total = (0, 0);
        for i in 1..=250 {
            let p = m.tick(&c, i as f64 / 250.).unwrap();
            total.0 += p.0;
            total.1 += p.1;
        }
        assert_eq!(total.0, -total.1);
        press(&mut s, &c, Action::Down);
        press(&mut s, &c, Action::Left);
        m.input(&s, &c, 1., None);
        assert_eq!(m.tick(&c, 1.004).unwrap(), (0, 0));
    }
    #[test]
    fn fractions_stop_and_reversal() {
        let mut c = Config::default();
        c.normal.min_speed = 40.;
        c.normal.max_speed = 40.;
        let mut s = InputState::default();
        let mut m = Movement::default();
        press(&mut s, &c, Action::Right);
        m.input(&s, &c, 0., None);
        for i in 1..=6 {
            assert_eq!(m.tick(&c, i as f64 * 0.004).unwrap(), (0, 0));
        }
        assert_eq!(m.tick(&c, 0.028).unwrap(), (1, 0));
        s.release_key(c.bindings[&Action::Right]);
        press(&mut s, &c, Action::Left);
        m.input(&s, &c, 0.028, None);
        assert_eq!(m.tick(&c, 0.032).unwrap(), (0, 0));
        s.cancel();
        m.input(&s, &c, 0.032, None);
        assert_eq!(m.tick(&c, 0.036).unwrap(), (0, 0));
    }
    #[test]
    fn precision_taps_delay_and_mode_entry() {
        let c = Config::default();
        let mut s = InputState::default();
        let mut m = Movement::default();
        press(&mut s, &c, Action::Precision);
        m.input(&s, &c, 0., None);
        press(&mut s, &c, Action::Right);
        assert_eq!(m.input(&s, &c, 0., Some(Action::Right)), (1, 0));
        press(&mut s, &c, Action::Up);
        assert_eq!(m.input(&s, &c, 0., Some(Action::Up)), (0, -1));
        for i in 1..=40 {
            assert_eq!(m.tick(&c, i as f64 * 0.004).unwrap(), (0, 0));
        }
        let mut sum = 0;
        for i in 41..=100 {
            sum += m.tick(&c, i as f64 * 0.004).unwrap().0;
        }
        assert!(sum > 0);
        let mut s = InputState::default();
        let mut m = Movement::default();
        press(&mut s, &c, Action::Right);
        m.input(&s, &c, 0., Some(Action::Right));
        press(&mut s, &c, Action::Precision);
        assert_eq!(m.input(&s, &c, 0.01, None), (0, 0));
    }
    #[test]
    fn priority_and_stall() {
        let c = Config::default();
        let mut s = InputState::default();
        let mut m = Movement::default();
        press(&mut s, &c, Action::Right);
        press(&mut s, &c, Action::Turbo);
        m.input(&s, &c, 0., None);
        assert_eq!(m.mode, Mode::Turbo);
        assert!(m.tick(&c, 0.004).unwrap().0 >= 3);
        press(&mut s, &c, Action::Precision);
        m.input(&s, &c, 0.005, None);
        assert_eq!(m.mode, Mode::Precision);
        assert!(m.tick(&c, 0.2).is_err());
        assert_eq!(m.tick(&c, 0.204).unwrap(), (0, 0));
    }
    #[test]
    fn jitter_preserves_elapsed_distance() {
        let mut c = Config::default();
        c.normal.min_speed = 1000.;
        c.normal.max_speed = 1000.;
        let mut s = InputState::default();
        press(&mut s, &c, Action::Right);
        let mut m = Movement::default();
        m.input(&s, &c, 0., None);
        let mut t = 0.;
        let mut sum = 0;
        for dt in [0.003, 0.006, 0.002, 0.009].into_iter().cycle().take(200) {
            t += dt;
            sum += m.tick(&c, t).unwrap().0;
        }
        assert!((sum as f64 - t * 1000.).abs() < 1.01);
    }
}
