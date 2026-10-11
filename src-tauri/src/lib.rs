mod calibration;
mod config;
mod cursor_tools;
#[cfg(feature = "diagnostics")]
mod diagnostics;
mod engine;
mod grid;
mod input;
mod locations;
mod magnifier;
mod movement;
#[cfg(test)]
mod movement_baseline;
#[cfg(test)]
mod movement_regression;
mod native;
mod profiles;
#[cfg(test)]
mod refinement_tests;
mod refinements;
mod scroll;
mod services;
pub(crate) fn diagnostic_dir() -> Option<std::path::PathBuf> {
    #[cfg(feature = "diagnostics")]
    if let Some(path) = std::env::var_os("KEYMOUSE_DIAGNOSTIC_DATA_DIR") {
        return Some(path.into());
    }
    None
}
use config::{Config, Store};
use engine::{Engine, Status};
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    Manager,
};
use tauri_plugin_autostart::ManagerExt;
pub struct AppState {
    services: Mutex<Option<services::Services>>,
    service_error: Mutex<Option<String>>,
    history: Mutex<locations::History>,
    displays: Mutex<Vec<locations::Display>>,
    config: Mutex<Config>,
    store: Store,
    engine: Engine,
    grid: Mutex<Option<grid::Session>>,
    grid_operation: Mutex<()>,
    operation: Mutex<()>,
    warning: Mutex<Option<String>>,
}
#[derive(serde::Serialize)]
struct Snapshot {
    service_error: Option<String>,
    config: Config,
    status: Status,
    warning: Option<String>,
    version: &'static str,
}
fn snapshot(state: &AppState) -> Snapshot {
    Snapshot {
        service_error: state
            .service_error
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone(),
        config: state
            .config
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone(),
        status: state
            .engine
            .status
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone(),
        warning: state
            .warning
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone(),
        version: env!("CARGO_PKG_VERSION"),
    }
}
#[tauri::command]
fn get_snapshot(state: tauri::State<AppState>) -> Snapshot {
    snapshot(&state)
}
#[tauri::command]
fn default_config() -> Config {
    Config::default()
}
#[tauri::command]
fn cursor_tools_snapshot(app: tauri::AppHandle) -> cursor_tools::Snapshot {
    cursor_tools::snapshot(&app)
}
#[tauri::command]
async fn cursor_tool(
    app: tauri::AppHandle,
    action: String,
    index: Option<usize>,
    name: Option<String>,
) -> Result<cursor_tools::Snapshot, String> {
    tauri::async_runtime::spawn_blocking(move || cursor_tools::command(&app, &action, index, name))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn calibration_snapshot(app: tauri::AppHandle) -> calibration::Snapshot {
    app.state::<AppState>().engine.calibration.snapshot()
}
#[tauri::command]
async fn calibration_command(
    app: tauri::AppHandle,
    action: String,
) -> Result<calibration::Snapshot, String> {
    tauri::async_runtime::spawn_blocking(move || match action.as_str() {
        "baseline" | "experimental" => calibration::begin(&app, &action),
        "target" => calibration::click(&app),
        "cancel" => {
            calibration::cancel(&app, true)?;
            Ok(app.state::<AppState>().engine.calibration.snapshot())
        }
        _ => Err("Unknown calibration action".into()),
    })
    .await
    .map_err(|e| e.to_string())?
}
fn update(app: &tauri::AppHandle, c: Config) -> Result<Snapshot, String> {
    let state = app.state::<AppState>();
    let _guard = state.operation.lock().map_err(|e| e.to_string())?;
    update_locked(app, c)
}
fn update_locked(app: &tauri::AppHandle, c: Config) -> Result<Snapshot, String> {
    c.validate().map_err(|e| e.to_string())?;
    let state = app.state::<AppState>();
    let old = state.config.lock().map_err(|e| e.to_string())?.clone();
    if !c.features.calibration.enabled
        || !c.enabled
        || (state
            .engine
            .calibration
            .mode
            .load(std::sync::atomic::Ordering::Acquire)
            != 0
            && c != old)
    {
        calibration::cancel(app, false)?;
    }
    let running = state
        .engine
        .status
        .lock()
        .map_err(|e| e.to_string())?
        .running;
    if running && state.grid.lock().map_err(|e| e.to_string())?.is_some() {
        grid::close(app)?;
    }
    let autostart = app.autolaunch();
    if c.start_at_login != old.start_at_login {
        if c.start_at_login {
            autostart.enable()
        } else {
            autostart.disable()
        }
        .map_err(|e| format!("Windows startup preference: {e}"))?;
    }
    let result = (|| {
        if running {
            let mode = state
                .engine
                .calibration
                .mode
                .load(std::sync::atomic::Ordering::Acquire);
            state.engine.apply(if mode != 0 {
                calibration::effective(&c, mode)
            } else {
                profiles::current(&c)
            })?;
        }
        state.store.save(&c).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if let Err(e) = result {
        let rollback = if running {
            state.engine.apply(profiles::current(&old))
        } else {
            Ok(())
        };
        let startup = if old.start_at_login {
            autostart.enable()
        } else {
            autostart.disable()
        };
        if rollback.is_err() || startup.is_err() {
            return Err(format!(
                "{e}; rollback failed — disable KeyMouse and restart"
            ));
        }
        return Err(e);
    }
    *state.config.lock().map_err(|e| e.to_string())? = c.clone();
    if let Some(service) = state.services.lock().map_err(|e| e.to_string())?.as_ref() {
        service.refresh();
    }
    *state.warning.lock().map_err(|e| e.to_string())? = if running {
        None
    } else {
        Some(
            "Settings saved. Quit and restart KeyMouse to retry native engine initialization."
                .into(),
        )
    };
    set_tray(app, c.enabled && running);
    Ok(snapshot(&state))
}
#[tauri::command]
async fn save_config(app: tauri::AppHandle, config: Config) -> Result<Snapshot, String> {
    tauri::async_runtime::spawn_blocking(move || update(&app, config))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn set_enabled(app: tauri::AppHandle, enabled: bool) -> Result<Snapshot, String> {
    tauri::async_runtime::spawn_blocking(move || enabled_update(&app, enabled))
        .await
        .map_err(|e| e.to_string())?
}
fn enabled_update(app: &tauri::AppHandle, enabled: bool) -> Result<Snapshot, String> {
    if enabled
        && !app
            .state::<AppState>()
            .engine
            .status
            .lock()
            .map_err(|e| e.to_string())?
            .running
    {
        return Err(
            "The input worker is unavailable. Correct settings, then quit and restart KeyMouse."
                .into(),
        );
    }
    let mut c = app
        .state::<AppState>()
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    c.enabled = enabled;
    update(app, c)
}
#[tauri::command]
async fn reset_config(app: tauri::AppHandle) -> Result<Snapshot, String> {
    tauri::async_runtime::spawn_blocking(move || update(&app, Config::default()))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn open_grid(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || grid::open(&app))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn grid_snapshot(state: tauri::State<AppState>) -> Option<grid::Session> {
    state.grid.lock().unwrap_or_else(|p| p.into_inner()).clone()
}
#[tauri::command]
async fn grid_command(
    app: tauri::AppHandle,
    action: String,
    region: Option<u8>,
) -> Result<Option<grid::Session>, String> {
    tauri::async_runtime::spawn_blocking(move || grid::command(&app, &action, region))
        .await
        .map_err(|e| e.to_string())?
}
fn show(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}
fn set_tray(app: &tauri::AppHandle, enabled: bool) {
    if let Some(t) = app.tray_by_id("keymouse") {
        let _ = t.set_tooltip(Some(if enabled {
            "KeyMouse — enabled"
        } else {
            "KeyMouse — disabled"
        }));
    }
}
pub(crate) fn emergency_disabled(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let _guard = state.operation.lock().unwrap_or_else(|p| p.into_inner());
    let mut c = state.config.lock().unwrap_or_else(|p| p.into_inner());
    c.enabled = false;
    if let Err(e) = state.store.save(&c) {
        *state.warning.lock().unwrap_or_else(|p| p.into_inner()) = Some(e.to_string());
    }
    drop(c);
    set_tray(app, false);
}
pub fn run() {
    let mut builder = tauri::Builder::default();
    if diagnostic_dir().is_none() {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _, _| show(app)));
    }
    let handler: Box<dyn Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync> =
        Box::new(tauri::generate_handler![
            get_snapshot,
            cursor_tools_snapshot,
            cursor_tool,
            calibration_snapshot,
            calibration_command,
            default_config,
            save_config,
            set_enabled,
            reset_config,
            open_grid,
            grid_snapshot,
            grid_command
        ]);
    #[cfg(feature = "diagnostics")]
    let diagnostic_handler: Box<dyn Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync> =
        Box::new(tauri::generate_handler![diagnostics::diagnostic]);
    #[cfg(feature = "diagnostics")]
    let handler = move |invoke: tauri::ipc::Invoke<tauri::Wry>| {
        if invoke.message.command() == "diagnostic" {
            diagnostic_handler(invoke)
        } else {
            handler(invoke)
        }
    };
    let mut context = tauri::generate_context!();
    if diagnostic_dir().is_some() {
        for window in &mut context.config_mut().app.windows {
            window.create = false;
        }
    }
    let built = builder
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .invoke_handler(handler)
        .setup(|app| {
            if let Some(dir) = diagnostic_dir() {
                let config = app
                    .config()
                    .app
                    .windows
                    .iter()
                    .find(|w| w.label == "main")
                    .ok_or("Missing diagnostic settings window")?
                    .clone();
                tauri::WebviewWindowBuilder::from_config(app, &config)?
                    .data_directory(dir.join("webview"))
                    .build()?;
            }
            let store = Store::new(&diagnostic_dir().unwrap_or(app.path().app_config_dir()?))?;
            let (mut config, mut warning) = store.load();
            match app.autolaunch().is_enabled() {
                Ok(enabled) => config.start_at_login = enabled,
                Err(e) => warning = Some(format!("Unable to inspect startup registration: {e}")),
            }
            let engine = Engine::start(config.clone(), app.handle().clone())
                .unwrap_or_else(Engine::unavailable);
            let running = engine
                .status
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .running;
            app.manage(AppState {
                services: Mutex::new(None),
                service_error: Mutex::new(None),
                history: Mutex::new(locations::History::default()),
                displays: Mutex::new(locations::displays(app.handle())?),
                config: Mutex::new(config.clone()),
                store,
                engine,
                grid: Mutex::new(None),
                grid_operation: Mutex::new(()),
                operation: Mutex::new(()),
                warning: Mutex::new(warning),
            });
            *app.state::<AppState>()
                .services
                .lock()
                .unwrap_or_else(|p| p.into_inner()) =
                Some(services::Services::start(app.handle().clone())?);
            let open = MenuItem::with_id(app, "open", "Open settings", true, None::<&str>)?;
            let toggle = MenuItem::with_id(app, "toggle", "Enable / disable", true, None::<&str>)?;
            let grid = MenuItem::with_id(app, "grid", "Grid navigation", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit KeyMouse", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &toggle, &grid, &quit])?;
            TrayIconBuilder::with_id("keymouse")
                .icon(
                    app.default_window_icon()
                        .ok_or("Missing application icon")?
                        .clone(),
                )
                .menu(&menu)
                .tooltip(if config.enabled && running {
                    "KeyMouse — enabled"
                } else {
                    "KeyMouse — disabled"
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show(app),
                    "quit" => {
                        let app = app.clone();
                        std::thread::spawn(move || {
                            let service = app
                                .state::<AppState>()
                                .services
                                .lock()
                                .unwrap_or_else(|p| p.into_inner())
                                .take();
                            if let Some(service) = service {
                                service.stop();
                            }
                            app.state::<AppState>().engine.stop();
                            app.exit(0);
                        });
                    }
                    "toggle" => {
                        let enabled = app
                            .state::<AppState>()
                            .config
                            .lock()
                            .unwrap_or_else(|p| p.into_inner())
                            .enabled;
                        let handle = app.clone();
                        std::thread::spawn(move || {
                            if let Err(e) = enabled_update(&handle, !enabled) {
                                *handle
                                    .state::<AppState>()
                                    .warning
                                    .lock()
                                    .unwrap_or_else(|p| p.into_inner()) = Some(e);
                                show(&handle);
                            }
                        });
                    }
                    "grid" => {
                        let handle = app.clone();
                        std::thread::spawn(move || {
                            if let Err(e) = grid::open(&handle) {
                                *handle
                                    .state::<AppState>()
                                    .warning
                                    .lock()
                                    .unwrap_or_else(|p| p.into_inner()) = Some(e);
                                show(&handle);
                            }
                        });
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(event, TrayIconEvent::DoubleClick { .. }) {
                        show(tray.app_handle());
                    }
                })
                .build(app)?;
            if !config.start_minimized || !running {
                show(app.handle());
            }
            Ok(())
        })
        .on_window_event(|w, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                if w.label() == "main" {
                    let _ = w.hide();
                } else if w.label() == "grid" {
                    grid::cancel_from_native(w.app_handle());
                } else if w.label() == "calibration" {
                    let app = w.app_handle().clone();
                    std::thread::spawn(move || {
                        let _ = calibration::cancel(&app, true);
                    });
                }
            }
            tauri::WindowEvent::Focused(false)
                if w.label() == "grid"
                    && w.is_visible().unwrap_or(false)
                    && !w.is_focused().unwrap_or(false) =>
            {
                grid::cancel_from_native(w.app_handle())
            }
            _ => {}
        })
        .build(context);
    let app = match built {
        Ok(app) => app,
        Err(error) => {
            native::startup_error(&error.to_string());
            return;
        }
    };
    app.run(|app, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            if let Some(service) = app
                .state::<AppState>()
                .services
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .as_ref()
            {
                service.stop();
            }
            app.state::<AppState>().engine.stop();
        }
    });
}
