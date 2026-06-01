use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriverStatus {
    Ok,
    Inaccessible,
    Unknown,
    UnknownProblem,
    Disabled,
    DriverError,
    RestartRequired,
    DisabledService,
    NotInstalled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParentGpu {
    Auto,
    Nvidia,
    Amd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    Landscape,
    Portrait,
    LandscapeFlipped,
    PortraitFlipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppTheme {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppLanguage {
    System,
    En,
    #[serde(rename = "zh-CN")]
    ZhCn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectiveLanguage {
    #[serde(rename = "en")]
    En,
    #[serde(rename = "zh-CN")]
    ZhCn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayMode {
    pub width: i32,
    pub height: i32,
    pub hz: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportedResolution {
    pub width: i32,
    pub height: i32,
    pub refresh_rates: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplaySummary {
    pub index: i32,
    pub identifier: i64,
    pub device_name: String,
    pub display_name: String,
    pub active: bool,
    pub current_mode: Option<DisplayMode>,
    pub current_orientation: Orientation,
    pub supported_resolutions: Vec<SupportedResolution>,
    pub unsupported_current_mode: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostSnapshot {
    pub revision: i64,
    pub status: DriverStatus,
    pub driver_version: String,
    pub max_displays: i32,
    pub displays: Vec<DisplaySummary>,
    pub custom_modes: Vec<DisplayMode>,
    pub parent_gpu: ParentGpu,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub launch_on_login: bool,
    pub close_to_tray: bool,
    pub start_minimized: bool,
    pub fallback_display: bool,
    pub keep_screen_on: bool,
    pub theme: AppTheme,
    pub language: AppLanguage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub host: HostSnapshot,
    pub settings: AppSettings,
    pub effective_language: EffectiveLanguage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetDisplayModeInput {
    pub index: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hz: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation: Option<Orientation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyAdminConfigInput {
    pub custom_modes: Vec<DisplayMode>,
    pub parent_gpu: ParentGpu,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveDisplayInput {
    #[serde(default)]
    pub index: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettingsPatch {
    pub launch_on_login: Option<bool>,
    pub close_to_tray: Option<bool>,
    pub start_minimized: Option<bool>,
    pub fallback_display: Option<bool>,
    pub keep_screen_on: Option<bool>,
    pub theme: Option<AppTheme>,
    pub language: Option<AppLanguage>,
}

impl Default for AppSettingsPatch {
    fn default() -> Self {
        Self {
            launch_on_login: None,
            close_to_tray: None,
            start_minimized: None,
            fallback_display: None,
            keep_screen_on: None,
            theme: None,
            language: None,
        }
    }
}

pub fn default_app_settings() -> AppSettings {
    AppSettings {
        launch_on_login: false,
        close_to_tray: true,
        start_minimized: false,
        fallback_display: false,
        keep_screen_on: false,
        theme: AppTheme::System,
        language: AppLanguage::System,
    }
}

pub fn empty_host_snapshot() -> HostSnapshot {
    HostSnapshot {
        revision: 0,
        status: DriverStatus::Unknown,
        driver_version: "pending".to_string(),
        max_displays: 0,
        displays: Vec::new(),
        custom_modes: Vec::new(),
        parent_gpu: ParentGpu::Auto,
    }
}

pub fn empty_app_snapshot() -> AppSnapshot {
    AppSnapshot {
        host: empty_host_snapshot(),
        settings: default_app_settings(),
        effective_language: EffectiveLanguage::En,
    }
}

/// Resolve the effective UI language from the user setting plus the system locale.
/// Mirrors `src/main/i18n.ts:resolveLanguage`.
pub fn resolve_effective_language(setting: AppLanguage, system_locale: &str) -> EffectiveLanguage {
    match setting {
        AppLanguage::En => EffectiveLanguage::En,
        AppLanguage::ZhCn => EffectiveLanguage::ZhCn,
        AppLanguage::System => {
            if system_locale.to_ascii_lowercase().starts_with("zh") {
                EffectiveLanguage::ZhCn
            } else {
                EffectiveLanguage::En
            }
        }
    }
}

/// Used by `applyAdminConfig` polling to detect when the host has accepted the new modes.
/// Sort key matches `src/main/app-driver.ts:normalizeModes`.
pub fn canonical_modes_key(modes: &[DisplayMode]) -> String {
    let mut sorted: Vec<&DisplayMode> = modes.iter().collect();
    sorted.sort_by(|a, b| {
        a.width
            .cmp(&b.width)
            .then(a.height.cmp(&b.height))
            .then(a.hz.cmp(&b.hz))
    });
    serde_json::to_string(&sorted).unwrap_or_default()
}

pub type ErrorDetails = HashMap<String, serde_json::Value>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_snapshot_camelcase_roundtrip() {
        let s = HostSnapshot {
            revision: 7,
            status: DriverStatus::NotInstalled,
            driver_version: "1.2.3".into(),
            max_displays: 4,
            displays: vec![DisplaySummary {
                index: 0,
                identifier: 12345,
                device_name: "VDD".into(),
                display_name: "Virtual".into(),
                active: true,
                current_mode: Some(DisplayMode {
                    width: 1920,
                    height: 1080,
                    hz: 60,
                }),
                current_orientation: Orientation::LandscapeFlipped,
                supported_resolutions: vec![],
                unsupported_current_mode: false,
            }],
            custom_modes: vec![DisplayMode {
                width: 2560,
                height: 1440,
                hz: 144,
            }],
            parent_gpu: ParentGpu::Nvidia,
        };

        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"driverVersion\":\"1.2.3\""));
        assert!(json.contains("\"status\":\"not_installed\""));
        assert!(json.contains("\"parentGpu\":\"nvidia\""));
        assert!(json.contains("\"currentOrientation\":\"landscape_flipped\""));
        assert!(json.contains("\"customModes\""));
        assert!(json.contains("\"maxDisplays\":4"));

        let back: HostSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn app_language_zh_cn_uses_hyphen() {
        let json = serde_json::to_string(&AppLanguage::ZhCn).unwrap();
        assert_eq!(json, "\"zh-CN\"");
        let back: AppLanguage = serde_json::from_str("\"zh-CN\"").unwrap();
        assert_eq!(back, AppLanguage::ZhCn);
    }

    #[test]
    fn effective_language_from_system_locale() {
        assert_eq!(
            resolve_effective_language(AppLanguage::System, "zh-CN"),
            EffectiveLanguage::ZhCn
        );
        assert_eq!(
            resolve_effective_language(AppLanguage::System, "zh-TW"),
            EffectiveLanguage::ZhCn
        );
        assert_eq!(
            resolve_effective_language(AppLanguage::System, "en-US"),
            EffectiveLanguage::En
        );
        assert_eq!(
            resolve_effective_language(AppLanguage::En, "zh-CN"),
            EffectiveLanguage::En
        );
        assert_eq!(
            resolve_effective_language(AppLanguage::ZhCn, "en-US"),
            EffectiveLanguage::ZhCn
        );
    }

    #[test]
    fn canonical_modes_sorted_by_width_height_hz() {
        let a = vec![
            DisplayMode {
                width: 1920,
                height: 1080,
                hz: 60,
            },
            DisplayMode {
                width: 1280,
                height: 720,
                hz: 60,
            },
            DisplayMode {
                width: 1920,
                height: 1080,
                hz: 30,
            },
        ];
        let b = vec![
            DisplayMode {
                width: 1280,
                height: 720,
                hz: 60,
            },
            DisplayMode {
                width: 1920,
                height: 1080,
                hz: 30,
            },
            DisplayMode {
                width: 1920,
                height: 1080,
                hz: 60,
            },
        ];
        assert_eq!(canonical_modes_key(&a), canonical_modes_key(&b));
    }

    #[test]
    fn snapshot_default_status_unknown() {
        let s = empty_host_snapshot();
        assert_eq!(s.status, DriverStatus::Unknown);
        assert_eq!(s.parent_gpu, ParentGpu::Auto);
        assert_eq!(s.driver_version, "pending");
    }
}
