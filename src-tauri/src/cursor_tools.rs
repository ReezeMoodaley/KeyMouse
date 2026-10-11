use crate::{
    locations::{self, Location},
    AppState,
};
use tauri::Manager;
#[derive(serde::Serialize)]
pub struct Snapshot {
    pub history: Vec<Location>,
    pub history_index: usize,
}
pub fn snapshot(app: &tauri::AppHandle) -> Snapshot {
    let state = app.state::<AppState>();
    let h = state.history.lock().unwrap_or_else(|p| p.into_inner());
    Snapshot {
        history: h.entries.iter().cloned().collect(),
        history_index: h.index,
    }
}
pub fn record(app: &tauri::AppHandle, p: (i32, i32), event: &str) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    if !state
        .config
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .features
        .history_enabled
    {
        return;
    }
    let d = state.displays.lock().unwrap_or_else(|p| p.into_inner());
    if let Ok(location) = Location::capture(event.into(), p, &d) {
        state
            .history
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .record(location);
    }
}
pub fn command(
    app: &tauri::AppHandle,
    action: &str,
    index: Option<usize>,
    name: Option<String>,
) -> Result<Snapshot, String> {
    let state = app.state::<AppState>();
    if action == "capture" {
        if !state
            .config
            .lock()
            .map_err(|e| e.to_string())?
            .features
            .bookmarks_enabled
        {
            return Err("Bookmarks are disabled".into());
        }
        // Countdown belongs to this command thread, not the input worker or configuration transaction.
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
    let _operation = state.operation.lock().map_err(|e| e.to_string())?;
    let displays = locations::displays(app)?;
    *state.displays.lock().map_err(|e| e.to_string())? = displays.clone();
    let mut c = state.config.lock().map_err(|e| e.to_string())?.clone();
    match action {
        "capture" => {
            if !c.features.bookmarks_enabled {
                return Err("Bookmarks are disabled".into());
            }
            let bookmark = Location::capture(
                name.unwrap_or_else(|| "Bookmark".into()),
                crate::native::cursor_position()?,
                &displays,
            )?;
            if !c.features.bookmarks_enabled {
                return Err("Bookmarks were disabled during capture".into());
            }
            c.bookmarks.push(bookmark);
            crate::update_locked(app, c)?;
        }
        "delete" => {
            if !c.features.bookmarks_enabled {
                return Err("Bookmarks are disabled".into());
            }
            let i = index.ok_or("Missing bookmark index")?;
            if i >= c.bookmarks.len() {
                return Err("Bookmark does not exist".into());
            }
            c.bookmarks.remove(i);
            crate::update_locked(app, c)?;
        }
        "bookmark" => {
            if !c.features.bookmarks_enabled {
                return Err("Bookmarks are disabled".into());
            }
            let b = c
                .bookmarks
                .get(index.ok_or("Missing bookmark index")?)
                .ok_or("Bookmark does not exist")?;
            let p = b.resolve(&displays)?;
            record(app, crate::native::cursor_position()?, "Before bookmark");
            state.engine.reposition(p.0, p.1)?;
            record(app, crate::native::cursor_position()?, "Bookmark");
        }
        "back" | "forward" => {
            if !c.features.history_enabled {
                return Err("Cursor history is disabled".into());
            }
            let (i, p) = {
                let h = state.history.lock().map_err(|e| e.to_string())?;
                let (i, b) = h.destination(if action == "back" { -1 } else { 1 })?;
                (i, b.resolve(&displays)?)
            };
            state.engine.reposition(p.0, p.1)?;
            state.history.lock().map_err(|e| e.to_string())?.index = i;
        }
        _ => return Err("Unknown cursor tool action".into()),
    }
    Ok(snapshot(app))
}
