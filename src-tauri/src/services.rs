use std::{
    sync::{mpsc, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};
use tauri::Manager;
enum Message {
    Refresh,
    Stop,
}
pub struct Services {
    tx: mpsc::Sender<Message>,
    join: Mutex<Option<JoinHandle<()>>>,
}
impl Services {
    pub fn start(app: tauri::AppHandle) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();
        let join = std::thread::Builder::new()
            .name("keymouse-desktop-services".into())
            .spawn(move || run(app, rx))
            .map_err(|e| e.to_string())?;
        Ok(Self {
            tx,
            join: Mutex::new(Some(join)),
        })
    }
    pub fn refresh(&self) {
        let _ = self.tx.send(Message::Refresh);
    }
    pub fn stop(&self) {
        let _ = self.tx.send(Message::Stop);
        if let Some(j) = self.join.lock().unwrap_or_else(|p| p.into_inner()).take() {
            let _ = j.join();
        }
    }
}
impl Drop for Services {
    fn drop(&mut self) {
        self.stop();
    }
}
fn run(app: tauri::AppHandle, rx: mpsc::Receiver<Message>) {
    use windows::Win32::UI::{
        HiDpi::{SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2},
        WindowsAndMessaging::{DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE},
    };
    unsafe {
        let _ = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    let mut last = None;
    let mut lens: Option<crate::magnifier::Lens> = None;
    let mut profile_due = Instant::now();
    let mut lens_failed = false;
    let mut display_due = Instant::now();
    let mut c = app
        .state::<crate::AppState>()
        .config
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    loop {
        let state = app.state::<crate::AppState>();
        if display_due <= Instant::now() {
            if let Ok(displays) = crate::locations::native_displays() {
                *state.displays.lock().unwrap_or_else(|p| p.into_inner()) = displays;
            }
            display_due = Instant::now() + Duration::from_secs(1);
        }
        if profile_due <= Instant::now() {
            let _operation = state.operation.lock().unwrap_or_else(|p| p.into_inner());
            c = state
                .config
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .clone();
            let calibration_mode = state
                .engine
                .calibration
                .mode
                .load(std::sync::atomic::Ordering::Acquire);
            let effective = if calibration_mode != 0 {
                crate::calibration::effective(&c, calibration_mode)
            } else {
                crate::profiles::current(&c)
            };
            if last.as_ref() != Some(&effective) {
                if state
                    .engine
                    .status
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .running
                {
                    if let Err(e) = state.engine.apply(effective.clone()) {
                        *state
                            .service_error
                            .lock()
                            .unwrap_or_else(|p| p.into_inner()) = Some(e);
                    }
                }
                last = Some(effective);
            }
            profile_due = Instant::now() + Duration::from_millis(200);
        }
        if c.features.magnifier.enabled && c.enabled {
            if lens
                .as_ref()
                .is_some_and(|l| !l.same_config(&c.features.magnifier))
            {
                lens = None;
            }
            if lens.is_none() && !lens_failed {
                match crate::magnifier::Lens::create(c.features.magnifier.clone()) {
                    Ok(l) => lens = Some(l),
                    Err(e) => {
                        lens_failed = true;
                        *state
                            .service_error
                            .lock()
                            .unwrap_or_else(|p| p.into_inner()) = Some(format!("Magnifier: {e}"));
                    }
                }
            }
            if let Some(l) = &lens {
                let visible = {
                    let s = state
                        .engine
                        .status
                        .lock()
                        .unwrap_or_else(|p| p.into_inner());
                    s.running && s.enabled && s.mode == "Precision"
                };
                if let Err(e) = l.update(
                    visible,
                    &state.displays.lock().unwrap_or_else(|p| p.into_inner()),
                ) {
                    *state
                        .service_error
                        .lock()
                        .unwrap_or_else(|p| p.into_inner()) = Some(format!("Magnifier: {e}"));
                }
            }
        } else {
            lens = None;
            lens_failed = false;
        }
        unsafe {
            let mut msg = MSG::default();
            for _ in 0..64 {
                if !PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        let wait = if c.features.magnifier.enabled && c.enabled {
            Some(Duration::from_millis(33))
        } else if c.features.profiles_enabled {
            Some(Duration::from_millis(200))
        } else {
            None
        };
        let message = match wait {
            Some(t) => rx.recv_timeout(t).ok(),
            None => rx.recv().ok(),
        };
        match message {
            Some(Message::Stop) => break,
            Some(Message::Refresh) => {
                profile_due = Instant::now();
                lens_failed = false;
                *state
                    .service_error
                    .lock()
                    .unwrap_or_else(|p| p.into_inner()) = None;
            }
            None if wait.is_none() => break,
            _ => {}
        }
    }
}
