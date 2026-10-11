use crate::{
    config::{Config, Curve, Precision},
    refinements::{Acceleration, Axes, Braking, Tap},
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Profile {
    pub name: String,
    pub executable: String,
    pub enabled: bool,
    pub normal: Option<Curve>,
    pub precision: Option<Precision>,
    pub turbo: Option<Curve>,
    pub tap: Option<Tap>,
    pub acceleration: Option<Acceleration>,
    pub braking: Option<Braking>,
    pub axes: Option<Axes>,
}
pub fn effective(global: &Config, executable: &str) -> Config {
    let mut c = global.clone();
    if !global.features.profiles_enabled {
        return c;
    }
    let normalize = |s: &str| s.replace('/', "\\").to_lowercase();
    let Some(p) = global
        .profiles
        .iter()
        .find(|p| p.enabled && normalize(&p.executable) == normalize(executable))
    else {
        return c;
    };
    if let Some(v) = &p.normal {
        c.normal = v.clone();
    }
    if let Some(v) = &p.precision {
        c.precision = v.clone();
    }
    if let Some(v) = &p.turbo {
        c.turbo = v.clone();
    }
    if let Some(v) = &p.tap {
        c.features.tap = v.clone();
        c.features.tap.enabled &= global.features.tap.enabled;
    }
    if let Some(v) = &p.acceleration {
        c.features.acceleration = v.clone();
        c.features.acceleration.enabled &= global.features.acceleration.enabled;
    }
    if let Some(v) = &p.braking {
        c.features.braking = v.clone();
        c.features.braking.enabled &= global.features.braking.enabled;
    }
    if let Some(v) = &p.axes {
        c.features.axes = v.clone();
        c.features.axes.enabled &= global.features.axes.enabled;
    }
    c
}
pub fn foreground_executable() -> Result<String, String> {
    use windows::{
        core::PWSTR,
        Win32::{
            Foundation::CloseHandle,
            System::Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
                PROCESS_QUERY_LIMITED_INFORMATION,
            },
            UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
        },
    };
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return Err("No foreground application".into());
        }
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
            .map_err(|e| e.to_string())?;
        let mut buffer = vec![0_u16; 32768];
        let mut len = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(process);
        result.map_err(|e| e.to_string())?;
        Ok(String::from_utf16_lossy(&buffer[..len as usize]))
    }
}
pub fn current(c: &Config) -> Config {
    if c.features.profiles_enabled {
        effective(c, &foreground_executable().unwrap_or_default())
    } else {
        c.clone()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn precedence_global_availability_and_no_stale_overrides() {
        let mut c = Config::default();
        c.features.profiles_enabled = true;
        let mut curve = c.normal.clone();
        curve.max_speed = 700.;
        let mut tap = c.features.tap.clone();
        tap.enabled = true;
        c.profiles.push(Profile {
            name: "Editor".into(),
            executable: "C:\\editor.exe".into(),
            enabled: true,
            normal: Some(curve),
            tap: Some(tap),
            ..Profile::default()
        });
        let e = effective(&c, "c:/EDITOR.exe");
        assert_eq!(e.normal.max_speed, 700.);
        assert!(!e.features.tap.enabled);
        assert_eq!(e.turbo, c.turbo);
        c.features.tap.enabled = true;
        assert!(effective(&c, "C:\\editor.exe").features.tap.enabled);
        assert_eq!(effective(&c, "C:\\other.exe"), c);
        c.features.profiles_enabled = false;
        assert_eq!(effective(&c, "C:\\editor.exe"), c);
    }
}
