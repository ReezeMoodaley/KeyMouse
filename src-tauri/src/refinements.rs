use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TapStrategy {
    #[default]
    Discrete,
    Responsive,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Tap {
    pub enabled: bool,
    pub strategy: TapStrategy,
    pub step: u32,
    pub hold_ms: f64,
}
impl Default for Tap {
    fn default() -> Self {
        Self {
            enabled: false,
            strategy: TapStrategy::Discrete,
            step: 1,
            hold_ms: 150.,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    #[default]
    Power,
    Linear,
    Smoothstep,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Acceleration {
    pub enabled: bool,
    pub strategy: Strategy,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Braking {
    pub enabled: bool,
    pub perpendicular: bool,
    pub window_ms: f64,
    pub cap: f64,
    pub recovery_ms: f64,
    pub minimum_speed: f64,
}
impl Default for Braking {
    fn default() -> Self {
        Self {
            enabled: false,
            perpendicular: false,
            window_ms: 180.,
            cap: 120.,
            recovery_ms: 350.,
            minimum_speed: 200.,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Axes {
    pub enabled: bool,
    pub transition_ms: f64,
}
impl Default for Axes {
    fn default() -> Self {
        Self {
            enabled: false,
            transition_ms: 80.,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Magnifier {
    pub enabled: bool,
    pub zoom: f64,
    pub size: u32,
}
impl Default for Magnifier {
    fn default() -> Self {
        Self {
            enabled: false,
            zoom: 3.,
            size: 240,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Calibration {
    pub enabled: bool,
    pub target_size: u32,
    pub distance: u32,
    pub trials: u32,
}
impl Default for Calibration {
    fn default() -> Self {
        Self {
            enabled: false,
            target_size: 20,
            distance: 500,
            trials: 5,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Features {
    pub tap: Tap,
    pub acceleration: Acceleration,
    pub braking: Braking,
    pub axes: Axes,
    pub magnifier: Magnifier,
    pub calibration: Calibration,
    pub history_enabled: bool,
    pub bookmarks_enabled: bool,
    pub profiles_enabled: bool,
}
impl Features {
    pub fn validate(&self) -> Result<(), String> {
        let range = |v: f64, lo: f64, hi: f64| v.is_finite() && (lo..=hi).contains(&v);
        if !(1..=10).contains(&self.tap.step)
            || !range(self.tap.hold_ms, 0., 2000.)
            || !range(self.braking.window_ms, 1., 1000.)
            || !range(self.braking.cap, 1., 10000.)
            || !range(self.braking.recovery_ms, 1., 2000.)
            || !range(self.braking.minimum_speed, 1., 10000.)
            || !range(self.axes.transition_ms, 1., 500.)
            || !range(self.magnifier.zoom, 1.5, 8.)
            || !(120..=600).contains(&self.magnifier.size)
            || !(4..=100).contains(&self.calibration.target_size)
            || !(10..=3840).contains(&self.calibration.distance)
            || !(1..=100).contains(&self.calibration.trials)
        {
            return Err("Refinement parameters are outside their supported ranges".into());
        }
        Ok(())
    }
}
