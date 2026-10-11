//! Presentation preferences only; never dispatched to the input engine.
use crate::config::Store;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{path::Path, sync::Mutex};
use tauri::{Emitter, Manager};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeState {
    version: u32,
    revision: u64,
    active: Value,
    customs: Vec<Value>,
}
fn default_theme() -> Value {
    serde_json::from_str(include_str!("../../src/theme-default.json")).expect("embedded theme")
}
impl Default for ThemeState {
    fn default() -> Self {
        Self {
            version: 1,
            revision: 0,
            active: default_theme(),
            customs: vec![],
        }
    }
}
fn exact(v: &Value, sample: &Value) -> bool {
    match (v.as_object(), sample.as_object()) {
        (Some(a), Some(b)) => a.len() == b.len() && b.keys().all(|k| a.contains_key(k)),
        _ => false,
    }
}
pub fn validate_theme(t: &Value) -> Result<(), String> {
    let base = default_theme();
    if !exact(t, &base) || t["version"] != 1 {
        return Err("Unsupported theme structure or version".into());
    }
    let id = t["id"].as_str().ok_or("Invalid theme identifier")?;
    let name = t["name"].as_str().ok_or("Invalid theme name")?;
    if id.is_empty()
        || id.len() > 80
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        || name.trim().is_empty()
        || name.chars().count() > 80
    {
        return Err("Invalid theme name or identifier".into());
    }
    if !exact(&t["colors"], &base["colors"])
        || !t["colors"].as_object().unwrap().values().all(|c| {
            c.as_str().is_some_and(|s| {
                s.len() == 7
                    && s.starts_with('#')
                    && s.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
            })
        })
    {
        return Err("Theme colors must be hexadecimal; CSS is not accepted".into());
    }
    let bounded = |v: &Value, min: f64, max: f64| {
        v.as_f64()
            .is_some_and(|n| n.is_finite() && n >= min && n <= max)
    };
    if !bounded(&t["radius"], 0., 16.)
        || !bounded(&t["fontSize"], 12., 18.)
        || !["compact", "comfortable", "spacious"]
            .iter()
            .any(|s| t["density"] == *s)
    {
        return Err("Invalid theme geometry".into());
    }
    let e = &t["effects"];
    if !exact(e, &base["effects"])
        || !["off", "subtle", "standard", "strong"]
            .iter()
            .any(|s| e["glow"] == *s)
        || !["distortion", "scanlines", "noise", "flicker"]
            .iter()
            .all(|s| e[*s].is_boolean())
        || !["intensity", "ghost", "misalignment", "bloom"]
            .iter()
            .all(|s| bounded(&e[*s], 0., 100.))
        || !bounded(&e["displacement"], 0., 1.)
    {
        return Err("Invalid CRT settings".into());
    }
    Ok(())
}
impl ThemeState {
    fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.customs.len() > 100 || self.revision > 9_007_199_254_740_991 {
            return Err("Invalid theme store version or size".into());
        }
        validate_theme(&self.active)?;
        let mut ids = std::collections::HashSet::new();
        for t in &self.customs {
            validate_theme(t)?;
            let id = t["id"].as_str().unwrap();
            if [
                "phosphor-green",
                "amber-terminal",
                "cyber-cyan",
                "monochrome",
            ]
            .contains(&id)
                || !ids.insert(id)
            {
                return Err("Custom themes cannot replace presets or duplicate identifiers".into());
            }
        }
        Ok(())
    }
}
pub struct Themes {
    state: Mutex<ThemeState>,
    store: Store,
    warning: Mutex<Option<String>>,
}
impl Themes {
    pub fn new(dir: &Path) -> Self {
        let store = Store {
            path: dir.join("themes.json"),
        };
        let (state, warning) = if store.path.exists() {
            match std::fs::read(&store.path)
                .map_err(|e| e.to_string())
                .and_then(|b| serde_json::from_slice::<ThemeState>(&b).map_err(|e| e.to_string()))
                .and_then(|s| {
                    s.validate()?;
                    Ok(s)
                }) {
                Ok(s) => (s, None),
                Err(e) => (
                    ThemeState::default(),
                    Some(format!(
                        "Theme file was preserved; safe defaults loaded: {e}"
                    )),
                ),
            }
        } else {
            (ThemeState::default(), None)
        };
        Self {
            state: Mutex::new(state),
            store,
            warning: Mutex::new(warning),
        }
    }
    fn save(&self, mut next: ThemeState) -> Result<ThemeState, String> {
        next.validate()?;
        let mut current = self.state.lock().map_err(|e| e.to_string())?;
        if next.revision != current.revision {
            return Err(
                "Theme settings changed in another window. Cancel to reload before applying."
                    .into(),
            );
        }
        next.revision = current
            .revision
            .checked_add(1)
            .ok_or("Theme revision overflow")?;
        let bytes = serde_json::to_vec_pretty(&next).map_err(|e| e.to_string())?;
        self.store
            .atomic_write(&self.store.path, &bytes)
            .map_err(|e| e.to_string())?;
        *current = next.clone();
        *self.warning.lock().map_err(|e| e.to_string())? = None;
        Ok(next)
    }
}
#[tauri::command]
pub fn theme_load(app: tauri::AppHandle) -> Result<ThemeState, String> {
    let themes = app.state::<Themes>();
    // Surface damaged preferences without overwriting the original file.
    if let Some(w) = themes.warning.lock().map_err(|e| e.to_string())?.as_ref() {
        return Err(w.clone());
    }
    themes
        .state
        .lock()
        .map(|s| s.clone())
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn theme_save(app: tauri::AppHandle, state: ThemeState) -> Result<ThemeState, String> {
    let state = app.state::<Themes>().save(state)?;
    app.emit("theme-state", &state).map_err(|e| e.to_string())?;
    Ok(state)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_css_versions_and_unbounded_effects() {
        let mut t = default_theme();
        validate_theme(&t).unwrap();
        t["colors"]["bg-app"] = Value::from("url(https://example.com)");
        assert!(validate_theme(&t).is_err());
        t = default_theme();
        t["effects"]["displacement"] = Value::from(2);
        assert!(validate_theme(&t).is_err());
        t = default_theme();
        t["version"] = Value::from(2);
        assert!(validate_theme(&t).is_err());
        t = default_theme();
        t["css"] = Value::from("body{}");
        assert!(validate_theme(&t).is_err());
    }
    #[test]
    fn persists_effects_customs_and_rejects_stale_writes() {
        let d = tempfile::tempdir().unwrap();
        let store = Themes::new(d.path());
        let mut next = ThemeState::default();
        next.active["effects"]["intensity"] = Value::from(70);
        let mut custom = default_theme();
        custom["id"] = Value::from("custom-test");
        next.customs.push(custom);
        let saved = store.save(next.clone()).unwrap();
        assert_eq!(saved.revision, 1);
        assert!(store.save(next).is_err());
        let loaded = Themes::new(d.path());
        let value = loaded.state.lock().unwrap();
        assert_eq!(value.active["effects"]["intensity"], 70);
        assert_eq!(value.customs.len(), 1);
    }
    #[test]
    fn corrupt_preferences_preserved() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("themes.json");
        std::fs::write(&path, b"broken").unwrap();
        let store = Themes::new(d.path());
        assert!(store.warning.lock().unwrap().is_some());
        assert_eq!(std::fs::read(path).unwrap(), b"broken");
    }
}
