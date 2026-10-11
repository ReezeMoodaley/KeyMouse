//! Read-only UI information and window routing, separate from movement.
use tauri::{Emitter, Manager};
#[tauri::command]
pub fn presentation_visibility(window: tauri::WebviewWindow) -> bool {
    window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(false)
}
#[tauri::command]
pub async fn open_mini(app: tauri::AppHandle) -> Result<(), String> {
    // WebView2 construction must not block a synchronous IPC handler on Windows.
    tauri::async_runtime::spawn_blocking(move || build_mini(app))
        .await
        .map_err(|e| e.to_string())?
}
fn build_mini(app: tauri::AppHandle) -> Result<(), String> {
    let window = if let Some(w) = app.get_webview_window("mini") {
        w
    } else {
        let mut builder = tauri::WebviewWindowBuilder::new(
            &app,
            "mini",
            tauri::WebviewUrl::App("index.html?mini=1".into()),
        )
        .title("KeyMouse · Mini Control Panel")
        .inner_size(360., 540.)
        .min_inner_size(320., 420.);
        if let Some(dir) = crate::diagnostic_dir() {
            builder = builder.data_directory(dir.join("webview"));
        }
        builder.build().map_err(|e| e.to_string())?
    };
    window.show().map_err(|e| e.to_string())?;
    window.unminimize().map_err(|e| e.to_string())?;
    window.emit("ui-visible", true).map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}
#[tauri::command]
pub fn open_page(app: tauri::AppHandle, page: String) -> Result<(), String> {
    if !["Overview", "Theme Studio", "Calibration"].contains(&page.as_str()) {
        return Err("Unsupported destination".into());
    }
    crate::show(&app);
    let w = app
        .get_webview_window("main")
        .ok_or("Settings window unavailable")?;
    w.emit("ui-visible", true).map_err(|e| e.to_string())?;
    w.emit("navigate", page).map_err(|e| e.to_string())
}
#[derive(serde::Serialize)]
pub struct PresentationSnapshot {
    executable: Option<String>,
    matching_profile: Option<String>,
    effective: crate::config::Config,
    displays: Vec<crate::locations::Display>,
    scale_factor: f64,
    calibration_active: bool,
}
#[tauri::command]
pub async fn presentation_snapshot(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<PresentationSnapshot, String> {
    let scale_factor = window.scale_factor().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<crate::AppState>();
        let c = state.config.lock().map_err(|e| e.to_string())?.clone();
        let executable = crate::profiles::foreground_executable().ok();
        let normalize = |s: &str| s.replace('/', "\\").to_lowercase();
        let calibration_mode = state
            .engine
            .calibration
            .mode
            .load(std::sync::atomic::Ordering::Acquire);
        let calibration_active = calibration_mode != 0;
        let matching_profile = if c.features.profiles_enabled && !calibration_active {
            executable.as_ref().and_then(|exe| {
                c.profiles
                    .iter()
                    .find(|p| p.enabled && normalize(&p.executable) == normalize(exe))
                    .map(|p| p.name.clone())
            })
        } else {
            None
        };
        let effective = if calibration_active {
            crate::calibration::effective(&c, calibration_mode)
        } else {
            crate::profiles::effective(&c, executable.as_deref().unwrap_or_default())
        };
        let displays = crate::locations::displays(&app)?;
        Ok(PresentationSnapshot {
            executable,
            matching_profile,
            effective,
            displays,
            scale_factor,
            calibration_active,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
