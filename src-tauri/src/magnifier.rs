use crate::{locations::Display, refinements::Magnifier};
use windows::{
    core::w,
    Win32::{
        Foundation::{COLORREF, HWND, RECT},
        UI::{Magnification::*, WindowsAndMessaging::*},
    },
};
pub fn rectangles(p: (i32, i32), d: &Display, c: &Magnifier) -> (RECT, (i32, i32)) {
    let source = (c.size as f64 / c.zoom).ceil() as i32;
    let left = (p.0 - source / 2).clamp(d.x, d.x + (d.width as i32 - source).max(0));
    let top = (p.1 - source / 2).clamp(d.y, d.y + (d.height as i32 - source).max(0));
    let x = (p.0 + 32).clamp(d.x, d.x + (d.width as i32 - c.size as i32).max(0));
    let y = (p.1 + 32).clamp(d.y, d.y + (d.height as i32 - c.size as i32).max(0));
    (
        RECT {
            left,
            top,
            right: left + source,
            bottom: top + source,
        },
        (x, y),
    )
}
pub struct Lens {
    host: HWND,
    child: HWND,
    config: Magnifier,
}
impl Lens {
    pub fn create(config: Magnifier) -> Result<Self, String> {
        // SAFETY: window ownership, API initialization, updates and destruction stay on one service thread.
        unsafe {
            MagInitialize().ok().map_err(|e| e.to_string())?;
            let mut lens = Self {
                host: HWND::default(),
                child: HWND::default(),
                config,
            };
            lens.host = CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TOPMOST
                    | WS_EX_NOACTIVATE
                    | WS_EX_TRANSPARENT
                    | WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!("KeyMouse precision magnifier"),
                WS_POPUP,
                0,
                0,
                lens.config.size as i32,
                lens.config.size as i32,
                None,
                None,
                None,
                None,
            )
            .map_err(|e| e.to_string())?;
            SetLayeredWindowAttributes(lens.host, COLORREF(0), 255, LWA_ALPHA)
                .map_err(|e| e.to_string())?;
            lens.child = CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TRANSPARENT,
                WC_MAGNIFIER,
                w!(""),
                WS_CHILD | WS_VISIBLE,
                0,
                0,
                lens.config.size as i32,
                lens.config.size as i32,
                Some(lens.host),
                None,
                None,
                None,
            )
            .map_err(|e| e.to_string())?;
            let z = lens.config.zoom as f32;
            let mut transform = MAGTRANSFORM {
                v: [z, 0., 0., 0., z, 0., 0., 0., 1.],
            };
            MagSetWindowTransform(lens.child, &mut transform)
                .ok()
                .map_err(|e| e.to_string())?;
            let mut excluded = [lens.host];
            MagSetWindowFilterList(lens.child, MW_FILTERMODE_EXCLUDE, 1, excluded.as_mut_ptr())
                .ok()
                .map_err(|e| e.to_string())?;
            Ok(lens)
        }
    }
    pub fn same_config(&self, c: &Magnifier) -> bool {
        self.config == *c
    }
    pub fn update(&self, visible: bool, displays: &[Display]) -> Result<(), String> {
        unsafe {
            if !visible {
                let _ = ShowWindow(self.host, SW_HIDE);
                return Ok(());
            }
            let p = crate::native::cursor_position()?;
            let d = displays
                .iter()
                .find(|d| {
                    p.0 >= d.x
                        && p.1 >= d.y
                        && (p.0 as i64) < d.x as i64 + d.width as i64
                        && (p.1 as i64) < d.y as i64 + d.height as i64
                })
                .ok_or("Cursor monitor unavailable")?;
            let mut config = self.config.clone();
            config.size = config.size.min(d.width).min(d.height);
            let (source, (x, y)) = rectangles(p, d, &config);
            MagSetWindowSource(self.child, source)
                .ok()
                .map_err(|e| e.to_string())?;
            SetWindowPos(
                self.host,
                Some(HWND_TOPMOST),
                x,
                y,
                config.size as i32,
                config.size as i32,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            )
            .map_err(|e| e.to_string())?;
            SetWindowPos(
                self.child,
                None,
                0,
                0,
                config.size as i32,
                config.size as i32,
                SWP_NOACTIVATE | SWP_NOZORDER,
            )
            .map_err(|e| e.to_string())?;
            let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(self.child), None, true);
            Ok(())
        }
    }
}
impl Drop for Lens {
    fn drop(&mut self) {
        unsafe {
            if !self.host.is_invalid() {
                let _ = DestroyWindow(self.host);
            }
            let _ = MagUninitialize();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn negative_monitor_bounds_and_zoom() {
        let d = Display {
            name: "test".into(),
            x: -1920,
            y: -100,
            width: 1920,
            height: 1080,
        };
        let (r, (x, y)) = rectangles((-1, 979), &d, &Magnifier::default());
        assert_eq!(r.right, 0);
        assert_eq!(r.bottom, 980);
        assert!(x >= d.x && x + 240 <= 0);
        assert!(y >= d.y && y + 240 <= 980);
    }
    #[test]
    fn native_lens_creation_and_resource_cleanup() {
        let before = unsafe { GetForegroundWindow() };
        let lens = Lens::create(Magnifier::default()).unwrap();
        let hwnd = lens.host;
        lens.update(true, &crate::locations::native_displays().unwrap())
            .unwrap();
        assert_eq!(unsafe { GetForegroundWindow() }, before);
        assert!(unsafe { IsWindow(Some(hwnd)) }.as_bool());
        drop(lens);
        assert!(!unsafe { IsWindow(Some(hwnd)) }.as_bool());
    }
}
