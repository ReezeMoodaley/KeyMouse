//! Optional diagnostic binary only; absent from normal builds and installers.
use tauri::Manager;
#[tauri::command]
pub async fn diagnostic(
    app: tauri::AppHandle,
    action: String,
    x: Option<i32>,
    y: Option<i32>,
    key: Option<u16>,
    down: Option<bool>,
) -> Result<serde_json::Value, String> {
    if crate::diagnostic_dir().is_none() {
        return Err("Not an isolated diagnostic instance".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<crate::AppState>();
        match action.as_str() {
            "position" => Ok(serde_json::json!(crate::native::cursor_position()?)),
            "foreground" => Ok(serde_json::json!(crate::profiles::foreground_executable()?)),
            "focus" => {
                let window = app
                    .get_webview_window("main")
                    .ok_or("Missing diagnostic window")?;
                window.show().map_err(|e| e.to_string())?;
                window.set_focus().map_err(|e| e.to_string())?;
                let position = window.inner_position().map_err(|e| e.to_string())?;
                Ok(serde_json::json!([position.x, position.y]))
            }
            "reposition" => {
                state
                    .engine
                    .reposition(x.ok_or("Missing x")?, y.ok_or("Missing y")?)?;
                Ok(serde_json::json!(true))
            }
            "key" => {
                let key = key.ok_or("Missing key")?;
                if ![0x7c, 0x7d, 0x7e, 0x7f, 0x80, 0x85, 0x86].contains(&key) {
                    return Err(
                        "Diagnostics permits only direction/left-button/precision/turbo controls"
                            .into(),
                    );
                }
                // Clicks are permitted only over our own disposable browser fixture.
                // Releases remain available if focus changes, so a failed test cannot leave a hold.
                if key == 0x80 && down == Some(true) {
                    use windows::Win32::{
                        System::Threading::GetCurrentProcessId,
                        UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
                    };
                    let mut pid = 0;
                    unsafe {
                        GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut pid));
                        if pid != GetCurrentProcessId() {
                            return Err(
                                "Diagnostic click requires its own foreground window".into()
                            );
                        }
                    }
                }
                state
                    .engine
                    .diagnostic_key(key, down.ok_or("Missing key state")?)?;
                Ok(serde_json::json!(true))
            }
            "quit" => {
                let app = app.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    let service = app
                        .state::<crate::AppState>()
                        .services
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .take();
                    if let Some(service) = service {
                        service.stop();
                    }
                    app.state::<crate::AppState>().engine.stop();
                    app.exit(0);
                });
                Ok(serde_json::json!(true))
            }
            _ => Err("Unknown diagnostic action".into()),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
