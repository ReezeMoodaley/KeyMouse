use crate::config::Config;
use serde::Serialize;
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
#[derive(Debug, Default, Clone, Serialize)]
pub struct Status {
    pub normal_max_speed: f64,
    pub running: bool,
    pub enabled: bool,
    pub moving: bool,
    pub mode: String,
    pub last_action: Option<String>,
    pub error: Option<String>,
    pub devices: usize,
    pub ticks: u64,
    pub stalls: u64,
    pub max_tick_ms: f64,
    pub dispatch_p95_ms: Option<f64>,
    pub dispatch_samples: usize,
}
pub enum Command {
    #[cfg(feature = "diagnostics")]
    DiagnosticKey(u16, bool, mpsc::Sender<Result<(), String>>),
    Reposition(i32, i32, mpsc::Sender<Result<(), String>>),
    Apply(Box<Config>, mpsc::Sender<Result<(), String>>),
    Pause(bool, mpsc::Sender<Result<(), String>>),
    Stop,
}
pub struct Engine {
    pub calibration: Arc<crate::calibration::Recorder>,
    tx: mpsc::Sender<Command>,
    hwnd: usize,
    join: Mutex<Option<JoinHandle<()>>>,
    pub status: Arc<Mutex<Status>>,
}
impl Engine {
    #[cfg(feature = "diagnostics")]
    pub fn diagnostic_key(&self, key: u16, down: bool) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.send(Command::DiagnosticKey(key, down, tx))?;
        rx.recv_timeout(std::time::Duration::from_secs(3))
            .map_err(|e| e.to_string())?
    }
    pub fn unavailable(error: String) -> Self {
        let (tx, _rx) = mpsc::channel();
        Self {
            calibration: Arc::new(crate::calibration::Recorder::default()),
            tx,
            hwnd: 0,
            join: Mutex::new(None),
            status: Arc::new(Mutex::new(Status {
                error: Some(error),
                ..Status::default()
            })),
        }
    }
    pub fn start(config: Config, app: tauri::AppHandle) -> Result<Self, String> {
        let calibration = Arc::new(crate::calibration::Recorder::default());
        let recorder = calibration.clone();
        let (tx, rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(Status::default()));
        let shared = status.clone();
        let join = std::thread::Builder::new()
            .name("keymouse-input-engine".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    crate::native::run(config, app, rx, shared.clone(), ready_tx, recorder)
                }));
                let mut s = shared.lock().unwrap_or_else(|p| p.into_inner());
                s.running = false;
                s.moving = false;
                s.enabled = false;
                match result {
                    Ok(Err(e)) => s.error = Some(e),
                    Err(_) => s.error = Some(
                        "Input worker panicked; owned-button cleanup was attempted during unwind"
                            .into(),
                    ),
                    _ => {}
                }
            })
            .map_err(|e| e.to_string())?;
        match ready_rx.recv_timeout(std::time::Duration::from_secs(10)) {
            Ok(Ok(hwnd)) => Ok(Self {
                calibration,
                tx,
                hwnd,
                join: Mutex::new(Some(join)),
                status,
            }),
            Ok(Err(e)) => {
                let _ = join.join();
                Err(e)
            }
            Err(e) => {
                let _ = tx.send(Command::Stop);
                let _ = join.join();
                Err(format!("Input worker initialization failed: {e}"))
            }
        }
    }
    fn send(&self, c: Command) -> Result<(), String> {
        self.tx
            .send(c)
            .map_err(|_| "Input worker unavailable".to_string())?;
        crate::native::wake(self.hwnd);
        Ok(())
    }
    pub fn apply(&self, c: Config) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.send(Command::Apply(Box::new(c), tx))?;
        rx.recv_timeout(std::time::Duration::from_secs(3))
            .map_err(|e| e.to_string())?
    }
    pub fn pause(&self, p: bool) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.send(Command::Pause(p, tx))?;
        rx.recv_timeout(std::time::Duration::from_secs(3))
            .map_err(|e| e.to_string())?
    }
    pub fn reposition(&self, x: i32, y: i32) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.send(Command::Reposition(x, y, tx))?;
        rx.recv_timeout(std::time::Duration::from_secs(3))
            .map_err(|e| e.to_string())?
    }
    pub fn stop(&self) {
        let _ = self.send(Command::Stop);
        if let Some(j) = self.join.lock().unwrap_or_else(|p| p.into_inner()).take() {
            let _ = j.join();
        }
    }
}
impl Drop for Engine {
    fn drop(&mut self) {
        self.stop();
    }
}
