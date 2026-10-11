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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Phase {
    #[default]
    Idle,
    InitialTap,
    WaitingForHold,
    ContinuousHold,
    Precision,
    Turbo,
    CorrectiveBraking,
}
#[derive(Default)]
pub struct Movement {
    pub mode: Mode,
    pub phase: Phase,
    direction: (i32, i32),
    started: f64,
    last: f64,
    fraction: (f64, f64),
    axis_started: [f64; 2],
    press_started: [f64; 2],
    velocity: (f64, f64),
    previous: (i32, i32),
    changed: f64,
    previous_speed: f64,
    brake_started: Option<f64>,
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
    pub fn directional(&self) -> bool {
        self.direction != (0, 0)
    }
    pub fn idle_tick(&mut self, now: f64) {
        self.last = now;
        self.phase = Phase::Idle;
        self.fraction = (0., 0.);
    }
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
        self.phase = if d == (0, 0) {
            Phase::Idle
        } else {
            match m {
                Mode::Precision => Phase::Precision,
                Mode::Turbo => Phase::Turbo,
                Mode::Normal => Phase::ContinuousHold,
            }
        };
        let old = self.direction;
        if mode_change {
            self.brake_started = None;
            self.previous = (0, 0);
        }
        if d != old {
            let opposing = c.features.braking.enabled
                && ((s.held(Action::Left) && s.held(Action::Right))
                    || (s.held(Action::Up) && s.held(Action::Down)));
            let prior = if old != (0, 0) { old } else { self.previous };
            let prior_speed = if !c.features.braking.enabled {
                0.
            } else if old != (0, 0) {
                self.velocity.0.hypot(self.velocity.1)
            } else {
                self.previous_speed
            };
            let dot = d.0 * prior.0 + d.1 * prior.1;
            if c.features.braking.enabled
                && !mode_change
                && !opposing
                && m != Mode::Precision
                && d != (0, 0)
                && prior != (0, 0)
                && (old != (0, 0) || now - self.changed <= c.features.braking.window_ms / 1000.)
                && prior_speed >= c.features.braking.minimum_speed
                && (dot < 0 || (dot == 0 && c.features.braking.perpendicular))
            {
                self.brake_started = Some(now);
            }
            if old != (0, 0) {
                self.previous = old;
                self.previous_speed = prior_speed;
            }
            if opposing {
                self.previous = (0, 0);
                self.previous_speed = 0.;
                self.brake_started = None;
            }
            self.changed = now;
            for (i, (a, b)) in [(old.0, d.0), (old.1, d.1)].into_iter().enumerate() {
                if a != b {
                    self.axis_started[i] = now;
                    self.press_started[i] = now;
                }
            }
            if d == (0, 0) {
                self.brake_started = None;
                self.velocity = (0., 0.);
            }
        }
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
            if mode_change {
                self.axis_started = [now; 2];
                self.press_started = [now; 2];
                self.velocity = (0., 0.);
            }
        }
        if m == Mode::Precision && !mode_change {
            if let Some(a) = new_direction {
                self.started = now;
                self.last = now;
                let n = c.precision.step as i32;
                self.phase = Phase::InitialTap;
                return match a {
                    Action::Left if d.0 < 0 => (-n, 0),
                    Action::Right if d.0 > 0 => (n, 0),
                    Action::Up if d.1 < 0 => (0, -n),
                    Action::Down if d.1 > 0 => (0, n),
                    _ => (0, 0),
                };
            }
        }
        if c.features.tap.enabled && m != Mode::Precision && !mode_change {
            if let Some(a) = new_direction {
                let n = c.features.tap.step as i32;
                self.phase = Phase::InitialTap;
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
    pub fn configure(&mut self, old: &Config, new: &Config, now: f64) {
        if old.features.braking != new.features.braking {
            self.brake_started = None;
            self.previous = (0, 0);
        }
        if old.features.axes != new.features.axes {
            self.axis_started = [self.started; 2];
        }
        // Preserve held controls, button ownership and fractions. No tap is emitted by configuration changes.
        self.last = now;
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
            self.phase = Phase::WaitingForHold;
            return Ok((0, 0));
        }
        self.phase = if self.brake_started.is_some() {
            Phase::CorrectiveBraking
        } else {
            match self.mode {
                Mode::Precision => Phase::Precision,
                Mode::Turbo => Phase::Turbo,
                Mode::Normal => Phase::ContinuousHold,
            }
        };
        let active_dt = dt.min((now - hold_start).max(0.));
        let discrete = c.features.tap.enabled
            && self.mode != Mode::Precision
            && c.features.tap.strategy == crate::refinements::TapStrategy::Discrete;
        let tap_delay = if discrete {
            c.features.tap.hold_ms / 1000.
        } else {
            0.
        };
        let (effective, axis_dt) = if discrete {
            let x = (now - self.press_started[0] - tap_delay).clamp(0., active_dt);
            let y = (now - self.press_started[1] - tap_delay).clamp(0., active_dt);
            (
                (
                    if x > 0. { self.direction.0 } else { 0 },
                    if y > 0. { self.direction.1 } else { 0 },
                ),
                (x, y),
            )
        } else {
            (self.direction, (active_dt, active_dt))
        };
        if effective == (0, 0) {
            self.phase = Phase::WaitingForHold;
            self.velocity = (0., 0.);
            return Ok((0, 0));
        }
        let length = ((effective.0 * effective.0 + effective.1 * effective.1) as f64).sqrt();
        let strategy = |elapsed: f64| {
            if !c.features.acceleration.enabled {
                return speed(curve, elapsed);
            }
            let u = (elapsed / (curve.ramp_ms / 1000.)).clamp(0., 1.);
            let u = match c.features.acceleration.strategy {
                crate::refinements::Strategy::Power => u.powf(curve.exponent),
                crate::refinements::Strategy::Linear => u,
                crate::refinements::Strategy::Smoothstep => u * u * (3. - 2. * u),
            };
            curve.min_speed + (curve.max_speed - curve.min_speed) * u
        };
        let base = strategy((now - hold_start - tap_delay).max(0.));
        let mut velocity = (
            effective.0 as f64 / length * base,
            effective.1 as f64 / length * base,
        );
        if c.features.axes.enabled {
            velocity = (
                effective.0 as f64
                    * strategy((now - self.axis_started[0] - delay - tap_delay).max(0.)),
                effective.1 as f64
                    * strategy((now - self.axis_started[1] - delay - tap_delay).max(0.)),
            );
            let magnitude = velocity.0.hypot(velocity.1);
            if magnitude > curve.max_speed {
                velocity.0 *= curve.max_speed / magnitude;
                velocity.1 *= curve.max_speed / magnitude;
            }
            let blend_x = (axis_dt.0 / (c.features.axes.transition_ms / 1000.)).clamp(0., 1.);
            let blend_y = (axis_dt.1 / (c.features.axes.transition_ms / 1000.)).clamp(0., 1.);
            velocity.0 = if effective.0 == 0 {
                0.
            } else {
                self.velocity.0 + (velocity.0 - self.velocity.0) * blend_x
            };
            velocity.1 = if effective.1 == 0 {
                0.
            } else {
                self.velocity.1 + (velocity.1 - self.velocity.1) * blend_y
            };
            // Reversal cannot carry old-direction inertia.
            if velocity.0 * (self.direction.0 as f64) < 0. {
                velocity.0 = 0.;
            }
            if velocity.1 * (self.direction.1 as f64) < 0. {
                velocity.1 = 0.;
            }
        }
        if let Some(start) = self.brake_started {
            let t = ((now - start) / (c.features.braking.recovery_ms / 1000.)).clamp(0., 1.);
            let cap = c.features.braking.cap.min(curve.max_speed)
                + (curve.max_speed - c.features.braking.cap.min(curve.max_speed))
                    * t
                    * t
                    * (3. - 2. * t);
            let magnitude = velocity.0.hypot(velocity.1);
            if magnitude > cap {
                velocity.0 *= cap / magnitude;
                velocity.1 *= cap / magnitude;
            }
            if t == 1. {
                self.brake_started = None;
            }
        }
        if c.features.axes.enabled {
            let magnitude = velocity.0.hypot(velocity.1);
            if magnitude > curve.max_speed {
                velocity.0 *= curve.max_speed / magnitude;
                velocity.1 *= curve.max_speed / magnitude;
            }
        }
        self.velocity = velocity;
        if !c.features.axes.enabled && self.brake_started.is_none() {
            // Preserve the v1 floating-point operation order as well as its formula.
            self.fraction.0 += effective.0 as f64 / length * (base * axis_dt.0);
            self.fraction.1 += effective.1 as f64 / length * (base * axis_dt.1);
        } else {
            self.fraction.0 += velocity.0 * axis_dt.0;
            self.fraction.1 += velocity.1 * axis_dt.1;
        }
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
