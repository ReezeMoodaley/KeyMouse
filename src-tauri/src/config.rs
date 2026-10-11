use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Validation(String),
    #[error("Configuration I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("Configuration JSON: {0}")]
    Json(#[from] serde_json::Error),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    LeftButton,
    RightButton,
    MiddleButton,
    ScrollPositive,
    ScrollNegative,
    Precision,
    Turbo,
    Horizontal,
}
impl Action {
    pub const ALL: [Self; 12] = [
        Self::Up,
        Self::Down,
        Self::Left,
        Self::Right,
        Self::LeftButton,
        Self::RightButton,
        Self::MiddleButton,
        Self::ScrollPositive,
        Self::ScrollNegative,
        Self::Precision,
        Self::Turbo,
        Self::Horizontal,
    ];
    pub fn direction(self) -> bool {
        matches!(self, Self::Up | Self::Down | Self::Left | Self::Right)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Curve {
    pub min_speed: f64,
    pub max_speed: f64,
    pub ramp_ms: f64,
    pub exponent: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Precision {
    #[serde(flatten)]
    pub curve: Curve,
    pub hold_ms: f64,
    pub step: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Scroll {
    pub repeat: bool,
    pub delay_ms: u32,
    pub interval_ms: u32,
    pub amount: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GridConfig {
    pub key: u16,
    pub depth: u8,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub profiles: Vec<crate::profiles::Profile>,
    #[serde(default)]
    pub bookmarks: Vec<crate::locations::Location>,
    #[serde(default)]
    pub features: crate::refinements::Features,
    pub version: u32,
    pub enabled: bool,
    pub bindings: BTreeMap<Action, u16>,
    pub normal: Curve,
    pub precision: Precision,
    pub turbo: Curve,
    pub scroll: Scroll,
    pub grid: GridConfig,
    pub start_minimized: bool,
    pub start_at_login: bool,
    pub theme: String,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            profiles: Vec::new(),
            bookmarks: Vec::new(),
            features: crate::refinements::Features::default(),
            version: 1,
            enabled: true,
            bindings: Action::ALL
                .into_iter()
                .enumerate()
                .map(|(i, a)| (a, 0x7c + i as u16))
                .collect(),
            normal: Curve {
                min_speed: 40.,
                max_speed: 1400.,
                ramp_ms: 900.,
                exponent: 1.6,
            },
            precision: Precision {
                curve: Curve {
                    min_speed: 8.,
                    max_speed: 80.,
                    ramp_ms: 700.,
                    exponent: 1.8,
                },
                hold_ms: 160.,
                step: 1,
            },
            turbo: Curve {
                min_speed: 900.,
                max_speed: 4000.,
                ramp_ms: 350.,
                exponent: 1.2,
            },
            scroll: Scroll {
                repeat: true,
                delay_ms: 300,
                interval_ms: 70,
                amount: 120,
            },
            grid: GridConfig {
                key: 0x47,
                depth: 3,
            },
            start_minimized: false,
            start_at_login: false,
            theme: "system".into(),
        }
    }
}
fn bounded(name: &str, value: f64, min: f64, max: f64) -> Result<(), Error> {
    if !value.is_finite() || !(min..=max).contains(&value) {
        return Err(Error::Validation(format!(
            "{name} must be between {min} and {max}"
        )));
    }
    Ok(())
}
impl Config {
    pub fn validate(&self) -> Result<(), Error> {
        if self.profiles.len() > 32 {
            return Err(Error::Validation(
                "At most 32 application profiles are supported".into(),
            ));
        }
        let mut paths = std::collections::BTreeSet::new();
        for p in &self.profiles {
            if p.name.trim().is_empty()
                || p.name.len() > 80
                || !std::path::Path::new(&p.executable).is_absolute()
                || !p.executable.to_lowercase().ends_with(".exe")
                || !paths.insert(p.executable.replace('/', "\\").to_lowercase())
            {
                return Err(Error::Validation(
                    "Profiles need a unique absolute executable path and a name".into(),
                ));
            }
            let mut candidate = self.clone();
            candidate.profiles.clear();
            candidate.features.profiles_enabled = true;
            candidate.profiles.push(p.clone());
            candidate.profiles[0].enabled = true;
            let mut resolved = crate::profiles::effective(&candidate, &p.executable);
            resolved.profiles.clear();
            // Validate override parameters even if their global availability is disabled.
            if let Some(t) = &p.tap {
                resolved.features.tap = t.clone();
            }
            if let Some(t) = &p.braking {
                resolved.features.braking = t.clone();
            }
            if let Some(t) = &p.axes {
                resolved.features.axes = t.clone();
            }
            resolved.validate()?;
        }
        if self.bookmarks.len() > 64 || self.bookmarks.iter().any(|b| !b.validate()) {
            return Err(Error::Validation("Invalid cursor bookmarks".into()));
        }
        self.features.validate().map_err(Error::Validation)?;
        if self.version != 1 {
            return Err(Error::Validation(
                "Unsupported configuration version".into(),
            ));
        }
        if self.bindings.len() != 12 {
            return Err(Error::Validation(
                "All twelve controls must be assigned".into(),
            ));
        }
        let mut used = std::collections::BTreeSet::new();
        for action in Action::ALL {
            let key = self
                .bindings
                .get(&action)
                .ok_or_else(|| Error::Validation(format!("Missing {action:?}")))?;
            if !(0x7c..=0x87).contains(key) {
                return Err(Error::Validation(
                    "Bindings must use F13–F24 to avoid ordinary typing and system shortcuts"
                        .into(),
                ));
            }
            if !used.insert(key) {
                return Err(Error::Validation(
                    "Each action needs a distinct binding".into(),
                ));
            }
        }
        for (name, c) in [
            ("Normal", &self.normal),
            ("Precision", &self.precision.curve),
            ("Turbo", &self.turbo),
        ] {
            bounded(&format!("{name} minimum speed"), c.min_speed, 1., 10000.)?;
            bounded(
                &format!("{name} maximum speed"),
                c.max_speed,
                c.min_speed,
                10000.,
            )?;
            bounded(&format!("{name} ramp duration"), c.ramp_ms, 1., 10000.)?;
            bounded(&format!("{name} exponent"), c.exponent, 0.1, 5.)?;
        }
        bounded("Precision hold delay", self.precision.hold_ms, 0., 2000.)?;
        bounded("Precision step", self.precision.step as f64, 1., 10.)?;
        bounded("Scroll delay", self.scroll.delay_ms as f64, 0., 2000.)?;
        bounded(
            "Scroll interval",
            self.scroll.interval_ms as f64,
            10.,
            1000.,
        )?;
        bounded("Scroll amount", self.scroll.amount as f64, 1., 1200.)?;
        if !(1..=6).contains(&self.grid.depth) {
            return Err(Error::Validation("Grid depth must be 1–6".into()));
        }
        if !(0x41..=0x5a).contains(&self.grid.key) && !(0x70..=0x7b).contains(&self.grid.key) {
            return Err(Error::Validation(
                "Grid shortcut must be Ctrl+Alt with A–Z or F1–F12".into(),
            ));
        }
        if !["system", "dark", "light"].contains(&self.theme.as_str()) {
            return Err(Error::Validation("Unknown theme".into()));
        }
        Ok(())
    }
}
pub struct Store {
    pub path: PathBuf,
}
impl Store {
    pub fn new(dir: &Path) -> Result<Self, Error> {
        fs::create_dir_all(dir)?;
        Ok(Self {
            path: dir.join("config.json"),
        })
    }
    pub fn load(&self) -> (Config, Option<String>) {
        if !self.path.exists() && !self.path.with_extension("last-good.json").exists() {
            return (Config::default(), None);
        }
        let read = |path: &Path| -> Result<Config, Error> {
            let c: Config = serde_json::from_slice(&fs::read(path)?)?;
            c.validate()?;
            Ok(c)
        };
        match read(&self.path) {
            Ok(c) => (c, None),
            Err(e) => match read(&self.path.with_extension("last-good.json")) {
                Ok(c) => (
                    c,
                    Some(format!("Recovered last-known-good configuration: {e}")),
                ),
                Err(_) => (
                    Config::default(),
                    Some(format!(
                        "Using safe defaults: {e}. Original file was preserved."
                    )),
                ),
            },
        }
    }
    pub fn save(&self, c: &Config) -> Result<(), Error> {
        c.validate()?;
        let bytes = serde_json::to_vec_pretty(c)?;
        self.atomic_write(&self.path.with_extension("last-good.json"), &bytes)?;
        self.atomic_write(&self.path, &bytes)
    }
    fn atomic_write(&self, path: &Path, bytes: &[u8]) -> Result<(), Error> {
        let tmp = path.with_extension("tmp");
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows::{
                core::PCWSTR,
                Win32::Storage::FileSystem::{
                    MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
                },
            };
            let from: Vec<u16> = tmp.as_os_str().encode_wide().chain(Some(0)).collect();
            let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            // SAFETY: both strings are terminated and live until the call returns.
            unsafe {
                MoveFileExW(
                    PCWSTR(from.as_ptr()),
                    PCWSTR(to.as_ptr()),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )
            }
            .map_err(|e| Error::Io(std::io::Error::other(e.to_string())))?;
        }
        #[cfg(not(windows))]
        fs::rename(tmp, path)?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_roundtrip() {
        let c = Config::default();
        c.validate().unwrap();
        assert_eq!(
            c,
            serde_json::from_str::<Config>(&serde_json::to_string(&c).unwrap()).unwrap()
        );
    }
    #[test]
    fn invalid_values() {
        let mut c = Config::default();
        c.normal.max_speed = f64::NAN;
        assert!(c.validate().is_err());
        c = Config::default();
        c.bindings.insert(Action::Up, 0x41);
        assert!(c.validate().is_err());
        c = Config::default();
        c.bindings.insert(Action::Up, 0x7d);
        assert!(c.validate().is_err());
        c = Config::default();
        c.version = 9;
        assert!(c.validate().is_err());
    }
    #[test]
    fn recovery() {
        let d = tempfile::tempdir().unwrap();
        let s = Store::new(d.path()).unwrap();
        let mut c = Config::default();
        c.normal.max_speed = 1234.;
        s.save(&c).unwrap();
        fs::write(&s.path, b"broken").unwrap();
        let (r, w) = s.load();
        assert_eq!(r, c);
        assert!(w.is_some());
    }
}
