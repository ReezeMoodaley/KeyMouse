use crate::{
    config::{Action, Config},
    input::InputState,
    movement::{Movement, Phase},
    refinements::{Strategy, TapStrategy},
};
#[test]
fn perpendicular_braking_is_separate_and_mode_change_cancels_it() {
    for perpendicular in [false, true] {
        let mut c = Config::default();
        c.features.braking.enabled = true;
        c.features.braking.perpendicular = perpendicular;
        let mut s = InputState::default();
        let mut m = Movement::default();
        key(&mut s, &c, &mut m, Action::Turbo, true, 0.);
        key(&mut s, &c, &mut m, Action::Right, true, 0.);
        for i in 1..=150 {
            m.tick(&c, i as f64 * 0.004).unwrap();
        }
        key(&mut s, &c, &mut m, Action::Right, false, 0.601);
        key(&mut s, &c, &mut m, Action::Up, true, 0.61);
        m.tick(&c, 0.614).unwrap();
        assert_eq!(m.phase == Phase::CorrectiveBraking, perpendicular);
        key(&mut s, &c, &mut m, Action::Precision, true, 0.615);
        m.tick(&c, 0.619).unwrap();
        assert_ne!(m.phase, Phase::CorrectiveBraking);
    }
}
#[test]
fn all_movement_feature_combinations_stop_and_respect_modes() {
    for tap in [TapStrategy::Discrete, TapStrategy::Responsive] {
        for strategy in [Strategy::Power, Strategy::Linear, Strategy::Smoothstep] {
            for axes in [false, true] {
                for braking in [false, true] {
                    let mut c = Config::default();
                    c.features.tap.enabled = true;
                    c.features.tap.strategy = tap;
                    c.features.acceleration.enabled = true;
                    c.features.acceleration.strategy = strategy;
                    c.features.axes.enabled = axes;
                    c.features.braking.enabled = braking;
                    let mut s = InputState::default();
                    let mut m = Movement::default();
                    let mut seed = 13_u64;
                    for i in 0..3000 {
                        let now = i as f64 * 0.004;
                        if i % 29 == 0 {
                            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                            let a = [
                                Action::Right,
                                Action::Left,
                                Action::Up,
                                Action::Down,
                                Action::Precision,
                                Action::Turbo,
                            ][(seed >> 24) as usize % 6];
                            key(&mut s, &c, &mut m, a, seed & 1 == 0, now);
                        }
                        let p = m.tick(&c, now).unwrap();
                        let d = crate::movement::direction(&s);
                        if d.0 == 0 {
                            assert_eq!(p.0, 0);
                        } else {
                            assert!(p.0 * d.0 >= 0);
                        }
                        if d.1 == 0 {
                            assert_eq!(p.1, 0);
                        } else {
                            assert!(p.1 * d.1 >= 0);
                        }
                        let max = match crate::movement::mode(&s) {
                            crate::movement::Mode::Normal => c.normal.max_speed,
                            crate::movement::Mode::Turbo => c.turbo.max_speed,
                            crate::movement::Mode::Precision => c.precision.curve.max_speed,
                        };
                        assert!((p.0 as f64).hypot(p.1 as f64) <= max * 0.004 + 1.42);
                    }
                    s.cancel();
                    m.input(&s, &c, 12., None);
                    assert_eq!(m.tick(&c, 12.004).unwrap(), (0, 0));
                }
            }
        }
    }
}
fn key(
    s: &mut InputState,
    c: &Config,
    m: &mut Movement,
    a: Action,
    down: bool,
    now: f64,
) -> (i32, i32) {
    let t = s.transition(1, c.bindings[&a], down, c);
    m.input(
        s,
        c,
        now,
        t.and_then(|(a, down)| (down && a.direction()).then_some(a)),
    )
}
#[test]
fn tap_strategies_and_precision_isolation() {
    for strategy in [TapStrategy::Discrete, TapStrategy::Responsive] {
        let mut c = Config::default();
        c.features.tap.enabled = true;
        c.features.tap.strategy = strategy;
        c.normal.min_speed = 1000.;
        c.normal.max_speed = 1000.;
        let mut s = InputState::default();
        let mut m = Movement::default();
        assert_eq!(key(&mut s, &c, &mut m, Action::Right, true, 0.), (1, 0));
        assert_eq!(m.phase, Phase::InitialTap);
        assert_eq!(
            m.tick(&c, 0.004).unwrap(),
            if strategy == TapStrategy::Discrete {
                (0, 0)
            } else {
                (4, 0)
            }
        );
        let mut total = 0;
        for i in 2..=50 {
            total += m.tick(&c, i as f64 * 0.004).unwrap().0;
        }
        assert!(total > 0);
        key(&mut s, &c, &mut m, Action::Right, false, 0.201);
        assert_eq!(m.tick(&c, 0.205).unwrap(), (0, 0));
        key(&mut s, &c, &mut m, Action::Precision, true, 0.21);
        assert_eq!(key(&mut s, &c, &mut m, Action::Right, true, 0.22), (1, 0));
        assert_eq!(m.tick(&c, 0.224).unwrap(), (0, 0));
    }
}
#[test]
fn discrete_new_axis_waits_without_stopping_existing_axis_or_replaying_tap() {
    let mut c = Config::default();
    c.features.tap.enabled = true;
    c.normal.min_speed = 1000.;
    c.normal.max_speed = 1000.;
    let mut s = InputState::default();
    let mut m = Movement::default();
    assert_eq!(key(&mut s, &c, &mut m, Action::Right, true, 0.), (1, 0));
    for i in 1..=250 {
        m.tick(&c, i as f64 * 0.004).unwrap();
    }
    assert_eq!(key(&mut s, &c, &mut m, Action::Down, true, 1.), (0, 1));
    for i in 1..=35 {
        let p = m.tick(&c, 1. + i as f64 * 0.004).unwrap();
        assert!(p.0 > 0);
        assert_eq!(p.1, 0);
    }
    assert!(m.tick(&c, 1.152).unwrap().1 > 0);
    key(&mut s, &c, &mut m, Action::Down, false, 1.153);
    assert_eq!(m.tick(&c, 1.157).unwrap().1, 0);
}
#[test]
fn acceleration_strategy_has_runtime_effect_and_disable_restores_power() {
    let mut c = Config::default();
    c.features.acceleration.enabled = true;
    let sequence = |c: &Config| {
        let mut s = InputState::default();
        let mut m = Movement::default();
        key(&mut s, c, &mut m, Action::Right, true, 0.);
        (1..=100)
            .map(|i| m.tick(c, i as f64 * 0.004).unwrap().0)
            .sum::<i32>()
    };
    let original = sequence(&Config::default());
    c.features.acceleration.strategy = Strategy::Linear;
    assert!(sequence(&c) > original);
    c.features.acceleration.enabled = false;
    assert_eq!(sequence(&c), original);
    c.features.acceleration.enabled = true;
    c.features.acceleration.strategy = Strategy::Smoothstep;
    assert_ne!(sequence(&c), original);
}
#[test]
fn braking_only_for_fast_corrections_and_recovers() {
    let mut c = Config::default();
    c.features.braking.enabled = true;
    let mut s = InputState::default();
    let mut m = Movement::default();
    key(&mut s, &c, &mut m, Action::Turbo, true, 0.);
    key(&mut s, &c, &mut m, Action::Right, true, 0.);
    for i in 1..=150 {
        m.tick(&c, i as f64 * 0.004).unwrap();
    }
    // Opposing held controls cancel, not a correction.
    key(&mut s, &c, &mut m, Action::Left, true, 0.601);
    assert_eq!(m.tick(&c, 0.605).unwrap(), (0, 0));
    key(&mut s, &c, &mut m, Action::Left, false, 0.606);
    for i in 1..=100 {
        m.tick(&c, 0.606 + i as f64 * 0.004).unwrap();
    }
    key(&mut s, &c, &mut m, Action::Right, false, 1.007);
    key(&mut s, &c, &mut m, Action::Left, true, 1.02);
    let p = m.tick(&c, 1.024).unwrap();
    assert!(p.0.abs() <= 1);
    assert_eq!(m.phase, Phase::CorrectiveBraking);
    let mut last = 0;
    for i in 2..=100 {
        last = m.tick(&c, 1.02 + i as f64 * 0.004).unwrap().0.abs();
    }
    assert!(last > 3);
    key(&mut s, &c, &mut m, Action::Left, false, 1.421);
    assert_eq!(m.tick(&c, 1.425).unwrap(), (0, 0));
}
#[test]
fn per_axis_transitions_are_bounded_and_release_is_immediate() {
    let mut c = Config::default();
    c.features.axes.enabled = true;
    let mut s = InputState::default();
    let mut m = Movement::default();
    key(&mut s, &c, &mut m, Action::Right, true, 0.);
    let mut before = 0;
    for i in 1..=250 {
        before = m.tick(&c, i as f64 * 0.004).unwrap().0;
    }
    key(&mut s, &c, &mut m, Action::Down, true, 1.);
    let p = m.tick(&c, 1.004).unwrap();
    assert!((p.0 - before).abs() <= 1);
    assert!(p.1.abs() <= 1);
    for i in 2..=500 {
        let p = m.tick(&c, 1. + i as f64 * 0.004).unwrap();
        assert!((p.0 as f64).hypot(p.1 as f64) <= c.normal.max_speed * 0.004 + 1.42);
    }
    key(&mut s, &c, &mut m, Action::Down, false, 3.001);
    assert_eq!(m.tick(&c, 3.005).unwrap().1, 0);
    key(&mut s, &c, &mut m, Action::Right, false, 3.006);
    key(&mut s, &c, &mut m, Action::Left, true, 3.007);
    assert!(m.tick(&c, 3.011).unwrap().0 <= 0);
    key(&mut s, &c, &mut m, Action::Left, false, 3.012);
    assert_eq!(m.tick(&c, 3.016).unwrap(), (0, 0));
}
#[test]
fn migration_toggle_settings_and_buttons() {
    let mut c = Config::default();
    let mut json = serde_json::to_value(&c).unwrap();
    json.as_object_mut().unwrap().remove("features");
    json.as_object_mut().unwrap().remove("bookmarks");
    json.as_object_mut().unwrap().remove("profiles");
    assert_eq!(serde_json::from_value::<Config>(json).unwrap(), c);
    c.features.tap.enabled = true;
    c.features.tap.step = 7;
    let old = c.clone();
    c.features.tap.enabled = false;
    let mut s = InputState::default();
    let mut m = Movement::default();
    key(&mut s, &old, &mut m, Action::LeftButton, true, 0.);
    s.commit_button(0, true);
    key(&mut s, &old, &mut m, Action::Right, true, 0.);
    m.configure(&old, &c, 0.01);
    assert!(s.held(Action::LeftButton));
    assert!(s.owned[0]);
    assert_eq!(c.features.tap.step, 7);
    assert_eq!(s.reconcile_buttons(), Vec::<(usize, bool)>::new());
}
#[test]
fn button_only_tick_after_long_idle_is_not_a_movement_stall() {
    let c = Config::default();
    let mut s = InputState::default();
    let mut m = Movement::default();
    key(&mut s, &c, &mut m, Action::LeftButton, true, 1000.);
    s.commit_button(0, true);
    assert!(!m.directional());
    m.idle_tick(1000.);
    assert_eq!(m.tick(&c, 1000.004).unwrap(), (0, 0));
    assert!(s.owned[0]);
    assert!(s.held(Action::LeftButton));
}
