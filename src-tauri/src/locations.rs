use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Display {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Location {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub display: Display,
    pub u: f64,
    pub v: f64,
}
impl Location {
    pub fn capture(name: String, p: (i32, i32), displays: &[Display]) -> Result<Self, String> {
        let d = displays
            .iter()
            .find(|d| {
                p.0 as i64 >= d.x as i64
                    && p.1 as i64 >= d.y as i64
                    && (p.0 as i64) < d.x as i64 + d.width as i64
                    && (p.1 as i64) < d.y as i64 + d.height as i64
            })
            .ok_or("Cursor is outside connected displays")?;
        Ok(Self {
            name,
            x: p.0,
            y: p.1,
            display: d.clone(),
            u: (p.0 as f64 - d.x as f64) / d.width as f64,
            v: (p.1 as f64 - d.y as f64) / d.height as f64,
        })
    }
    pub fn resolve(&self, displays: &[Display]) -> Result<(i32, i32), String> {
        let matches: Vec<_> = displays
            .iter()
            .filter(|d| {
                if self.display.name.is_empty() {
                    **d == self.display
                } else {
                    d.name == self.display.name
                }
            })
            .collect();
        if matches.len() != 1 {
            return Err("Saved display is missing or ambiguous; recapture this location".into());
        }
        let d = matches[0];
        if !self.u.is_finite()
            || !self.v.is_finite()
            || !(0.0..1.0).contains(&self.u)
            || !(0.0..1.0).contains(&self.v)
            || d.width == 0
            || d.height == 0
        {
            return Err("Invalid saved monitor-relative position".into());
        }
        Ok((
            d.x.saturating_add(
                (self.u * d.width as f64)
                    .round()
                    .clamp(0., (d.width - 1) as f64) as i32,
            ),
            d.y.saturating_add(
                (self.v * d.height as f64)
                    .round()
                    .clamp(0., (d.height - 1) as f64) as i32,
            ),
        ))
    }
    pub fn validate(&self) -> bool {
        !self.name.trim().is_empty()
            && self.name.len() <= 80
            && self.display.width > 0
            && self.display.height > 0
            && self.u.is_finite()
            && self.v.is_finite()
            && (0.0..1.0).contains(&self.u)
            && (0.0..1.0).contains(&self.v)
    }
}
#[derive(Default)]
pub struct History {
    pub entries: VecDeque<Location>,
    pub index: usize,
}
impl History {
    pub fn record(&mut self, location: Location) {
        if self
            .entries
            .get(self.index)
            .is_some_and(|p| p.x == location.x && p.y == location.y)
        {
            return;
        }
        self.entries.truncate(self.index + 1);
        if self.entries.len() == 64 {
            self.entries.pop_front();
        }
        self.entries.push_back(location);
        self.index = self.entries.len() - 1;
    }
    pub fn destination(&self, delta: i32) -> Result<(usize, &Location), String> {
        let index = self.index as i64 + delta as i64;
        if !(0..self.entries.len() as i64).contains(&index) {
            return Err("No further cursor history".into());
        }
        Ok((index as usize, &self.entries[index as usize]))
    }
}
pub fn displays(_app: &tauri::AppHandle) -> Result<Vec<Display>, String> {
    native_displays()
}
pub fn native_displays() -> Result<Vec<Display>, String> {
    use windows::{
        core::BOOL,
        Win32::{
            Foundation::{LPARAM, RECT},
            Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW},
        },
    };
    unsafe extern "system" fn collect(m: HMONITOR, _dc: HDC, _r: *mut RECT, data: LPARAM) -> BOOL {
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        // SAFETY: EnumDisplayMonitors synchronously invokes this callback with a live Vec pointer.
        unsafe {
            if GetMonitorInfoW(m, &mut info.monitorInfo).as_bool() {
                let rect = info.monitorInfo.rcMonitor;
                let n = info
                    .szDevice
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(info.szDevice.len());
                let displays = &mut *(data.0 as *mut Vec<Display>);
                displays.push(Display {
                    name: String::from_utf16_lossy(&info.szDevice[..n]),
                    x: rect.left,
                    y: rect.top,
                    width: (rect.right - rect.left) as u32,
                    height: (rect.bottom - rect.top) as u32,
                });
            }
        }
        BOOL(1)
    }
    let mut displays = Vec::<Display>::new();
    unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(collect),
            LPARAM((&mut displays as *mut Vec<Display>) as isize),
        )
        .ok()
        .map_err(|e| e.to_string())?;
    }
    if displays.is_empty() {
        return Err("No connected displays".into());
    }
    Ok(displays)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn display() -> Display {
        Display {
            name: "display-1".into(),
            x: -1920,
            y: 0,
            width: 1920,
            height: 1080,
        }
    }
    #[test]
    fn bookmarks_follow_known_monitor_and_reject_missing_or_ambiguous() {
        let d = display();
        let b = Location::capture("test".into(), (-960, 540), std::slice::from_ref(&d)).unwrap();
        let mut moved = d.clone();
        moved.x = 0;
        moved.width = 3840;
        assert_eq!(b.resolve(&[moved]), Ok((1920, 540)));
        assert!(b.resolve(&[]).is_err());
        assert!(b.resolve(&[d.clone(), d]).is_err());
    }
    #[test]
    fn history_navigation_does_not_record_and_forward_branch_is_replaced() {
        let mut h = History::default();
        let d = display();
        for x in [-1900, -1800, -1700] {
            h.record(Location::capture("stop".into(), (x, 40), std::slice::from_ref(&d)).unwrap());
        }
        let (i, _) = h.destination(-1).unwrap();
        h.index = i;
        assert_eq!(h.entries.len(), 3);
        h.record(Location::capture("teleport".into(), (-1600, 40), &[d]).unwrap());
        assert_eq!(h.entries.len(), 3);
        assert!(h.destination(1).is_err());
    }
}
