//! Frozen v1 oracle; explicit seconds are the fake monotonic clock.
use crate::{
    config::{Action, Config},
    input::InputState,
    movement::Movement,
    movement_baseline,
};
#[test]
fn disabled_features_match_v1_every_displacement() {
    let c = Config::default();
    let mut input = InputState::default();
    let mut old = movement_baseline::Movement::default();
    let mut new = Movement::default();
    let actions = [
        Action::Right,
        Action::Up,
        Action::Left,
        Action::Down,
        Action::Precision,
        Action::Turbo,
    ];
    let mut seed = 29_u64;
    let mut now = 0.;
    for i in 0..20_000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        now += [0.002, 0.004, 0.007, 0.009][(seed >> 32) as usize % 4];
        if i % 7 == 0 {
            let a = actions[(seed >> 24) as usize % actions.len()];
            let transition = input.transition(1, c.bindings[&a], seed & 1 == 0, &c);
            let press = transition.and_then(|(a, down)| (down && a.direction()).then_some(a));
            assert_eq!(
                old.input(&input, &c, now, press),
                new.input(&input, &c, now, press),
                "input {i}"
            );
        }
        assert_eq!(old.tick(&c, now), new.tick(&c, now), "tick {i}");
        if i % 997 == 0 {
            old.reset(now);
            new.reset(now);
        }
    }
}
#[test]
#[ignore = "explicit calculation timing; not a hardware latency test"]
fn calculation_timing_baseline_vs_disabled() {
    let c = Config::default();
    let mut input = InputState::default();
    input.transition(1, c.bindings[&Action::Right], true, &c);
    let mut baseline_times = Vec::new();
    let mut current_times = Vec::new();
    for _ in 0..7 {
        let mut old = movement_baseline::Movement::default();
        old.input(&input, &c, 0., None);
        let start = std::time::Instant::now();
        for i in 1..=100_000 {
            std::hint::black_box(
                old.tick(std::hint::black_box(&c), i as f64 * 0.004)
                    .unwrap(),
            );
        }
        baseline_times.push(start.elapsed().as_nanos() as f64 / 100_000.);
        let mut new = Movement::default();
        new.input(&input, &c, 0., None);
        let start = std::time::Instant::now();
        for i in 1..=100_000 {
            std::hint::black_box(
                new.tick(std::hint::black_box(&c), i as f64 * 0.004)
                    .unwrap(),
            );
        }
        current_times.push(start.elapsed().as_nanos() as f64 / 100_000.);
    }
    baseline_times.sort_by(f64::total_cmp);
    current_times.sort_by(f64::total_cmp);
    println!("Debug-build pure tick median: baseline {:.1} ns; disabled refinements {:.1} ns; 250 Hz calculated CPU time {:.4}% / {:.4}% (excludes all OS, input and scheduling costs)",baseline_times[3],current_times[3],baseline_times[3]*250./1e7,current_times[3]*250./1e7);
}
