use serde::Serialize;
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
impl Rect {
    pub fn select(self, n: u8) -> Result<Self, String> {
        if !(1..=9).contains(&n) {
            return Err("Choose a region from 1–9".into());
        }
        let col = (n - 1) % 3;
        let row = (n - 1) / 3;
        let x = (self.width as u64 * col as u64 / 3) as u32;
        let y = (self.height as u64 * row as u64 / 3) as u32;
        let right = (self.width as u64 * (col + 1) as u64 / 3) as u32;
        let bottom = (self.height as u64 * (row + 1) as u64 / 3) as u32;
        if right <= x || bottom <= y {
            return Err(
                "That region is smaller than one pixel. Press Enter to confirm the current region."
                    .into(),
            );
        }
        Ok(Self {
            x: self.x + x as i32,
            y: self.y + y as i32,
            width: right - x,
            height: bottom - y,
        })
    }
    pub fn center(self) -> (i32, i32) {
        (
            self.x + (self.width / 2) as i32,
            self.y + (self.height / 2) as i32,
        )
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct Session {
    pub monitors: Vec<Rect>,
    pub monitor: usize,
    pub region: Rect,
    pub level: u8,
    pub depth: u8,
}
pub fn open(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<crate::AppState>();
    let _operation = state.grid_operation.lock().map_err(|e| e.to_string())?;
    if state.grid.lock().map_err(|e| e.to_string())?.is_some() {
        return Ok(());
    }
    if !state.config.lock().map_err(|e| e.to_string())?.enabled {
        return Err("Enable KeyMouse before opening the grid".into());
    }
    let main = app
        .get_webview_window("main")
        .ok_or("Settings window unavailable")?;
    let monitors = main
        .available_monitors()
        .map_err(|e| e.to_string())?
        .iter()
        .map(|m| Rect {
            x: m.position().x,
            y: m.position().y,
            width: m.size().width,
            height: m.size().height,
        })
        .collect::<Vec<_>>();
    if monitors.is_empty() {
        return Err("No displays available".into());
    }
    let current = main.current_monitor().map_err(|e| e.to_string())?;
    let monitor = current
        .and_then(|m| {
            monitors
                .iter()
                .position(|r| r.x == m.position().x && r.y == m.position().y)
        })
        .unwrap_or(0);
    let depth = state.config.lock().map_err(|e| e.to_string())?.grid.depth;
    let s = Session {
        region: monitors[monitor],
        monitors,
        monitor,
        level: 0,
        depth,
    };
    state.engine.pause(true)?;
    let result = (|| {
        let window = if let Some(w) = app.get_webview_window("grid") {
            w
        } else {
            let mut builder =
                WebviewWindowBuilder::new(app, "grid", WebviewUrl::App("index.html?grid=1".into()));
            if let Some(dir) = crate::diagnostic_dir() {
                builder = builder.data_directory(dir.join("webview"));
            }
            builder
                .title("KeyMouse grid — focused selection")
                .transparent(true)
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .visible(false)
                .resizable(false)
                .build()
                .map_err(|e| e.to_string())?
        };
        window
            .set_position(PhysicalPosition::new(s.region.x, s.region.y))
            .map_err(|e| e.to_string())?;
        window
            .set_size(PhysicalSize::new(s.region.width, s.region.height))
            .map_err(|e| e.to_string())?;
        *state.grid.lock().map_err(|e| e.to_string())? = Some(s.clone());
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        window.emit("grid-state", &s).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() {
        *state.grid.lock().map_err(|e| e.to_string())? = None;
        if let Some(w) = app.get_webview_window("grid") {
            let _ = w.hide();
        }
        let _ = state.engine.pause(false);
    }
    result
}
pub fn cancel_from_native(app: &tauri::AppHandle) {
    let handle = app.clone();
    std::thread::spawn(move || {
        let _ = close(&handle);
    });
}
pub fn close(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<crate::AppState>();
    let _operation = state.grid_operation.lock().map_err(|e| e.to_string())?;
    close_inner(app)
}
fn close_inner(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<crate::AppState>();
    *state.grid.lock().map_err(|e| e.to_string())? = None;
    let hidden = if let Some(w) = app.get_webview_window("grid") {
        w.hide().map_err(|e| e.to_string())
    } else {
        Ok(())
    };
    let resumed = state.engine.pause(false);
    hidden.and(resumed)
}
pub fn command(
    app: &tauri::AppHandle,
    action: &str,
    n: Option<u8>,
) -> Result<Option<Session>, String> {
    if action == "cancel" {
        close(app)?;
        return Ok(None);
    }
    let state = app.state::<crate::AppState>();
    let _operation = state.grid_operation.lock().map_err(|e| e.to_string())?;
    let mut grid = state.grid.lock().map_err(|e| e.to_string())?;
    let s = grid.as_mut().ok_or("Grid is not active")?;
    match action {
        "select" => {
            s.region = s.region.select(n.ok_or("Missing region")?)?;
            s.level += 1;
        }
        "restart" => {
            s.region = s.monitors[s.monitor];
            s.level = 0;
        }
        "monitor" => {
            s.monitor = (s.monitor + 1) % s.monitors.len();
            s.region = s.monitors[s.monitor];
            s.level = 0;
        }
        "confirm" => {}
        _ => return Err("Unknown grid command".into()),
    }
    if action == "confirm" || s.level >= s.depth {
        let (x, y) = s.region.center();
        crate::cursor_tools::record(app, crate::native::cursor_position()?, "Before grid");
        state.engine.reposition(x, y)?;
        crate::cursor_tools::record(app, crate::native::cursor_position()?, "Grid");
        drop(grid);
        close_inner(app)?;
        return Ok(None);
    }
    let snapshot = s.clone();
    drop(grid);
    if let Some(w) = app.get_webview_window("grid") {
        if action == "monitor" {
            w.set_position(PhysicalPosition::new(snapshot.region.x, snapshot.region.y))
                .map_err(|e| e.to_string())?;
            w.set_size(PhysicalSize::new(
                snapshot.region.width,
                snapshot.region.height,
            ))
            .map_err(|e| e.to_string())?;
        }
        w.emit("grid-state", &snapshot).map_err(|e| e.to_string())?;
    }
    Ok(Some(snapshot))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn odd_and_negative_bounds() {
        let r = Rect {
            x: -1921,
            y: -100,
            width: 1921,
            height: 1081,
        };
        let first = r.select(1).unwrap();
        let last = r.select(9).unwrap();
        assert_eq!(first.x, r.x);
        assert_eq!(last.x + last.width as i32, 0);
        assert_eq!(last.y + last.height as i32, 981);
        let area: u64 = (1..=9)
            .map(|n| {
                let p = r.select(n).unwrap();
                p.width as u64 * p.height as u64
            })
            .sum();
        assert_eq!(area, r.width as u64 * r.height as u64);
    }
    #[test]
    fn tiny_regions() {
        let r = Rect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };
        assert!(r.select(1).is_err());
        assert_eq!(r.select(9).unwrap(), r);
        assert_eq!(r.center(), (0, 0));
    }
}
