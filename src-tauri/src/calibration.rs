use crate::{config::Config, locations::Display};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering},
        Mutex,
    },
    time::Instant,
};
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};
#[derive(Debug, Clone, serde::Serialize)]
pub struct TrialResult {
    pub group: String,
    pub elapsed_ms: f64,
    pub generated_path: f64,
    pub straight_distance: f64,
    pub efficiency: Option<f64>,
    pub corrections: u32,
    pub overshoots: u32,
    pub ambiguous: bool,
    pub teleports: u32,
    pub samples: u32,
    pub max_sample_gap_ms: f64,
    pub dropped_samples: u64,
    pub configuration: Config,
}
struct Trial {
    started: Instant,
    start: (i32, i32),
    target: (i32, i32),
    last: (i32, i32),
    previous_delta: (i32, i32),
    path: f64,
    corrections: u32,
    overshoots: u32,
    ambiguous: bool,
    teleports: u32,
    samples: u32,
    last_time: Instant,
    max_gap: f64,
}
impl Trial {
    fn sample(&mut self, before: (i32, i32), after: (i32, i32), now: Instant) {
        if before != self.last {
            self.ambiguous = true;
        }
        let delta = (after.0 - before.0, after.1 - before.1);
        self.path += (delta.0 as f64).hypot(delta.1 as f64);
        if delta.0 as i64 * self.previous_delta.0 as i64
            + delta.1 as i64 * (self.previous_delta.1 as i64)
            < 0
        {
            self.corrections += 1;
        }
        let axis = (self.target.0 - self.start.0, self.target.1 - self.start.1);
        let projection = |p: (i32, i32)| {
            (p.0 - self.target.0) as i64 * axis.0 as i64
                + (p.1 - self.target.1) as i64 * axis.1 as i64
        };
        if projection(before) <= 0 && projection(after) > 0 {
            self.overshoots += 1;
        }
        self.max_gap = self
            .max_gap
            .max(now.duration_since(self.last_time).as_secs_f64() * 1000.);
        self.last = after;
        if delta != (0, 0) {
            self.previous_delta = delta;
        }
        self.samples += 1;
        self.last_time = now;
    }
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct Session {
    pub display: Display,
    pub start: (i32, i32),
    pub target: (i32, i32),
    pub size: u32,
    pub measuring: bool,
    pub completed: u32,
    pub trials: u32,
    pub group: String,
    pub configuration: Config,
}
#[derive(Default)]
struct Data {
    session: Option<Session>,
    trial: Option<Trial>,
    results: Vec<TrialResult>,
}
#[derive(Default)]
pub struct Recorder {
    pub active: AtomicBool,
    pub mode: AtomicU8,
    dropped: AtomicU64,
    data: Mutex<Data>,
}
#[derive(Clone, serde::Serialize)]
pub struct Snapshot {
    pub session: Option<Session>,
    pub results: Vec<TrialResult>,
}
impl Recorder {
    pub fn snapshot(&self) -> Snapshot {
        let d = self.data.lock().unwrap_or_else(|p| p.into_inner());
        Snapshot {
            session: d.session.clone(),
            results: d.results.clone(),
        }
    }
    pub fn sample(&self, before: (i32, i32), after: (i32, i32), now: Instant) {
        if !self.active.load(Ordering::Relaxed) {
            return;
        }
        if let Ok(mut d) = self.data.try_lock() {
            if let Some(t) = &mut d.trial {
                t.sample(before, after, now);
            }
        } else {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
    pub fn teleport(&self) {
        if self.active.load(Ordering::Relaxed) {
            let mut d = self.data.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(t) = &mut d.trial {
                t.teleports += 1;
                t.previous_delta = (0, 0);
            }
        }
    }
    pub fn stop(&self) {
        self.active.store(false, Ordering::Release);
        self.mode.store(0, Ordering::Release);
        let mut d = self.data.lock().unwrap_or_else(|p| p.into_inner());
        d.trial = None;
        d.session = None;
    }
    fn click(&self, p: (i32, i32), now: Instant) -> Result<bool, String> {
        let mut d = self.data.lock().map_err(|e| e.to_string())?;
        let session = d.session.as_ref().ok_or("No calibration session")?.clone();
        let point = if session.measuring {
            session.target
        } else {
            session.start
        };
        if (p.0 - point.0).unsigned_abs() > session.size / 2
            || (p.1 - point.1).unsigned_abs() > session.size / 2
        {
            return Err("Cursor is outside the target".into());
        }
        if !session.measuring {
            d.trial = Some(Trial {
                started: now,
                start: p,
                target: session.target,
                last: p,
                previous_delta: (0, 0),
                path: 0.,
                corrections: 0,
                overshoots: 0,
                ambiguous: false,
                teleports: 0,
                samples: 0,
                last_time: now,
                max_gap: 0.,
            });
            d.session.as_mut().unwrap().measuring = true;
            self.dropped.store(0, Ordering::Relaxed);
            self.active.store(true, Ordering::Release);
            return Ok(false);
        }
        self.active.store(false, Ordering::Release);
        let trial = d.trial.take().ok_or("Trial is unavailable")?;
        let ambiguous = trial.ambiguous || p != trial.last;
        let dropped = self.dropped.load(Ordering::Relaxed);
        let straight = (p.0 - trial.start.0) as f64;
        let straight = straight.hypot((p.1 - trial.start.1) as f64);
        let efficiency = if !ambiguous && trial.teleports == 0 && dropped == 0 && trial.path > 0. {
            Some((straight / trial.path).min(1.))
        } else {
            None
        };
        if d.results.len() == 200 {
            d.results.remove(0);
        }
        d.results.push(TrialResult {
            group: session.group,
            elapsed_ms: now.duration_since(trial.started).as_secs_f64() * 1000.,
            generated_path: trial.path,
            straight_distance: straight,
            efficiency,
            corrections: trial.corrections,
            overshoots: trial.overshoots,
            ambiguous,
            teleports: trial.teleports,
            samples: trial.samples,
            max_sample_gap_ms: trial.max_gap,
            dropped_samples: dropped,
            configuration: session.configuration,
        });
        let s = d.session.as_mut().unwrap();
        s.completed += 1;
        s.measuring = false;
        std::mem::swap(&mut s.start, &mut s.target);
        Ok(s.completed >= s.trials)
    }
}
pub fn effective(c: &Config, mode: u8) -> Config {
    let mut c = c.clone();
    if mode == 1 {
        c.features.tap.enabled = false;
        c.features.acceleration.enabled = false;
        c.features.braking.enabled = false;
        c.features.axes.enabled = false;
    }
    c
}
pub fn cancel(app: &tauri::AppHandle, restore: bool) -> Result<(), String> {
    let state = app.state::<crate::AppState>();
    let _guard = if restore {
        Some(state.operation.lock().map_err(|e| e.to_string())?)
    } else {
        None
    };
    let state = app.state::<crate::AppState>();
    state.engine.calibration.stop();
    if let Some(w) = app.get_webview_window("calibration") {
        w.hide().map_err(|e| e.to_string())?;
    }
    if restore {
        let c = state.config.lock().map_err(|e| e.to_string())?.clone();
        state.engine.apply(crate::profiles::current(&c))?;
    }
    Ok(())
}
pub fn begin(app: &tauri::AppHandle, group: &str) -> Result<Snapshot, String> {
    let state = app.state::<crate::AppState>();
    let _operation = state.operation.lock().map_err(|e| e.to_string())?;
    let c = state.config.lock().map_err(|e| e.to_string())?.clone();
    if !c.features.calibration.enabled || !c.enabled {
        return Err("Enable KeyMouse and calibration, then save settings first".into());
    }
    if !["baseline", "experimental"].contains(&group) {
        return Err("Choose baseline or experimental".into());
    }
    crate::grid::close(app)?;
    cancel(app, false)?;
    let displays = crate::locations::displays(app)?;
    let p = crate::native::cursor_position()?;
    let display = displays
        .into_iter()
        .find(|d| {
            p.0 >= d.x
                && p.1 >= d.y
                && (p.0 as i64) < d.x as i64 + d.width as i64
                && (p.1 as i64) < d.y as i64 + d.height as i64
        })
        .ok_or("Cursor display unavailable")?;
    let settings = &c.features.calibration;
    if display.width < settings.distance + settings.target_size * 2
        || display.height < settings.target_size * 2
    {
        return Err("Requested target distance does not fit on the cursor's display; reduce the distance or choose a larger display".into());
    }
    let x = display.x + ((display.width - settings.distance) / 2) as i32;
    let y = display.y + (display.height / 2) as i32;
    let mode = if group == "baseline" { 1 } else { 2 };
    let resolved = effective(&c, mode);
    let session = Session {
        start: (x, y),
        target: (x + settings.distance as i32, y),
        display: display.clone(),
        size: settings.target_size,
        measuring: false,
        completed: 0,
        trials: settings.trials,
        group: group.into(),
        configuration: resolved.clone(),
    };
    state.engine.apply(resolved)?;
    state.engine.calibration.mode.store(mode, Ordering::Release);
    state
        .engine
        .calibration
        .data
        .lock()
        .map_err(|e| e.to_string())?
        .session = Some(session);
    let result = (|| {
        let window = if let Some(w) = app.get_webview_window("calibration") {
            w
        } else {
            let mut builder = WebviewWindowBuilder::new(
                app,
                "calibration",
                WebviewUrl::App("index.html?calibration=1".into()),
            );
            if let Some(dir) = crate::diagnostic_dir() {
                builder = builder.data_directory(dir.join("webview"));
            }
            builder
                .title("KeyMouse calibration")
                .transparent(true)
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .visible(false)
                .focused(false)
                .focusable(false)
                .resizable(false)
                .build()
                .map_err(|e| e.to_string())?
        };
        window
            .set_position(PhysicalPosition::new(display.x, display.y))
            .map_err(|e| e.to_string())?;
        window
            .set_size(PhysicalSize::new(display.width, display.height))
            .map_err(|e| e.to_string())?;
        window.show().map_err(|e| e.to_string())?;
        window
            .emit("calibration-state", state.engine.calibration.snapshot())
            .map_err(|e| e.to_string())?;
        Ok(())
    })();
    if let Err(e) = result {
        let _ = cancel(app, false);
        let _ = state.engine.apply(crate::profiles::current(&c));
        return Err(e);
    }
    Ok(state.engine.calibration.snapshot())
}
pub fn click(app: &tauri::AppHandle) -> Result<Snapshot, String> {
    let state = app.state::<crate::AppState>();
    let done = state
        .engine
        .calibration
        .click(crate::native::cursor_position()?, Instant::now())?;
    if done {
        cancel(app, true)?;
    }
    let snapshot = state.engine.calibration.snapshot();
    if let Some(w) = app.get_webview_window("calibration") {
        w.emit("calibration-state", &snapshot)
            .map_err(|e| e.to_string())?;
    }
    Ok(snapshot)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sampling_is_explicit_and_teleports_are_excluded() {
        let r = Recorder::default();
        let t = Instant::now();
        r.sample((0, 0), (10, 0), t);
        assert!(r.snapshot().results.is_empty());
        let c = Config::default();
        r.data.lock().unwrap().session = Some(Session {
            display: Display {
                name: "test".into(),
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            start: (10, 20),
            target: (110, 20),
            size: 20,
            measuring: false,
            completed: 0,
            trials: 2,
            group: "baseline".into(),
            configuration: c,
        });
        r.click((10, 20), t).unwrap();
        r.sample((10, 20), (60, 20), t + std::time::Duration::from_millis(10));
        r.teleport();
        r.sample(
            (100, 20),
            (110, 20),
            t + std::time::Duration::from_millis(20),
        );
        r.click((110, 20), t + std::time::Duration::from_millis(30))
            .unwrap();
        let s = r.snapshot();
        assert_eq!(s.results[0].generated_path, 60.);
        assert!(s.results[0].efficiency.is_none());
        assert!(s.results[0].ambiguous);
        assert_eq!(s.results[0].teleports, 1);
        assert!(!r.active.load(Ordering::Relaxed));
        r.stop();
        assert!(r.snapshot().session.is_none());
    }
    #[test]
    fn baseline_disables_only_movement_experiments() {
        let mut c = Config::default();
        c.features.tap.enabled = true;
        c.features.magnifier.enabled = true;
        let e = effective(&c, 1);
        assert!(!e.features.tap.enabled);
        assert!(e.features.magnifier.enabled);
        assert_eq!(e.normal, c.normal);
        assert_eq!(effective(&c, 2), c);
    }
}
