use crate::common::Curve;

use super::common::{self, CommonSize};
use cosmic_text::Color;
use serde::{Deserialize, Deserializer};
use serde_jsonrc::Value;
use util::color::parse_color;
use way_edges_derive::GetSize;

#[derive(Debug, Deserialize, GetSize)]
pub struct WorkspaceConfig {
    #[serde(flatten)]
    // flatten does not support `default` yet.
    // issue: https://github.com/serde-rs/serde/issues/1626
    // PR: https://github.com/serde-rs/serde/pull/2687
    // #[serde(default = "dt_size")]
    pub size: CommonSize,

    #[serde(default = "dt_gap")]
    pub gap: i32,
    #[serde(default = "dt_active_increase")]
    pub active_increase: f64,

    #[serde(default = "dt_workspace_transition_duration")]
    pub workspace_transition_duration: u64,
    #[serde(default)]
    pub animation_curve: Curve,

    #[serde(default = "dt_pop_duration")]
    pub pop_duration: u64,

    #[serde(default = "dt_default_color")]
    #[serde(deserialize_with = "common::color_translate")]
    pub default_color: Color,
    #[serde(default = "dt_focus_color")]
    #[serde(deserialize_with = "common::color_translate")]
    pub focus_color: Color,
    #[serde(default = "dt_active_color")]
    #[serde(deserialize_with = "common::color_translate")]
    pub active_color: Color,
    #[serde(default)]
    #[serde(deserialize_with = "common::option_color_translate")]
    pub hover_color: Option<Color>,

    #[serde(default)]
    pub invert_direction: bool,

    #[serde(default)]
    pub output_name: Option<String>,

    pub preset: WorkspacePreset,
}

fn dt_gap() -> i32 {
    5
}
fn dt_active_increase() -> f64 {
    0.5
}
fn dt_workspace_transition_duration() -> u64 {
    300
}
fn dt_pop_duration() -> u64 {
    1000
}

fn dt_default_color() -> Color {
    parse_color("#003049").unwrap()
}
fn dt_focus_color() -> Color {
    parse_color("#669bbc").unwrap()
}
fn dt_active_color() -> Color {
    parse_color("#aaa").unwrap()
}

#[derive(Debug)]
pub enum WorkspacePreset {
    Hyprland,
    Niri(NiriConf),
}
impl<'de> Deserialize<'de> for WorkspacePreset {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;

        if let Some(preset_str) = value.as_str() {
            match preset_str {
                "hyprland" => Ok(WorkspacePreset::Hyprland),
                "niri" => Ok(WorkspacePreset::Niri(NiriConf::default())),
                _ => Err(serde::de::Error::unknown_variant(
                    preset_str,
                    &["hyprland", "niri"],
                )),
            }
        } else {
            #[derive(Deserialize)]
            #[serde(rename_all = "snake_case", tag = "type")]
            enum Helper {
                Hyprland,
                Niri(NiriConf),
            }

            let helper: Helper = Helper::deserialize(value).map_err(|err| {
                serde::de::Error::custom(format!("Failed to deserialize as object: {}", err))
            })?;

            match helper {
                Helper::Hyprland => Ok(WorkspacePreset::Hyprland),
                Helper::Niri(conf) => Ok(WorkspacePreset::Niri(conf)),
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct NiriConf {
    #[serde(default = "dt_filter_empty")]
    pub filter_empty: bool,
}
impl Default for NiriConf {
    fn default() -> Self {
        Self {
            filter_empty: dt_filter_empty(),
        }
    }
}

fn dt_filter_empty() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_jsonrc;
    use std::fmt;

    // for test
    impl fmt::Display for WorkspacePreset {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                WorkspacePreset::Hyprland => write!(f, "Hyprland"),
                WorkspacePreset::Niri(conf) => {
                    write!(f, "Niri(filter_empty: {})", conf.filter_empty)
                }
            }
        }
    }

    #[test]
    fn test_deserialize_string_niri() {
        let yaml_str = r#"{ "preset": "niri" }"#;
        let config: serde_jsonrc::Value = serde_jsonrc::from_str(yaml_str).unwrap();
        let preset: WorkspacePreset = serde_jsonrc::from_value(config["preset"].clone()).unwrap();
        assert_eq!(preset.to_string(), "Niri(filter_empty: true)");
    }

    #[test]
    fn test_deserialize_object_niri() {
        let yaml_str = r#"{ "preset": { "type": "niri" } }"#;
        let config: serde_jsonrc::Value = serde_jsonrc::from_str(yaml_str).unwrap();
        let preset: WorkspacePreset = serde_jsonrc::from_value(config["preset"].clone()).unwrap();
        assert_eq!(preset.to_string(), "Niri(filter_empty: true)");
    }

    #[test]
    fn test_deserialize_object_niri_with_config() {
        let yaml_str = r#"{ "preset": { "type": "niri", "filter_empty": false } }"#;
        let config: serde_jsonrc::Value = serde_jsonrc::from_str(yaml_str).unwrap();
        let preset: WorkspacePreset = serde_jsonrc::from_value(config["preset"].clone()).unwrap();
        assert_eq!(preset.to_string(), "Niri(filter_empty: false)");
    }

    #[test]
    fn test_deserialize_string_hyprland() {
        let yaml_str = r#"{ "preset": "hyprland" }"#;
        let config: serde_jsonrc::Value = serde_jsonrc::from_str(yaml_str).unwrap();
        let preset: WorkspacePreset = serde_jsonrc::from_value(config["preset"].clone()).unwrap();
        assert_eq!(preset.to_string(), "Hyprland");
    }

    #[test]
    fn test_deserialize_object_hyprland() {
        let yaml_str = r#"{ "preset": { "type": "hyprland" } }"#;
        let config: serde_jsonrc::Value = serde_jsonrc::from_str(yaml_str).unwrap();
        let preset: WorkspacePreset = serde_jsonrc::from_value(config["preset"].clone()).unwrap();
        assert_eq!(preset.to_string(), "Hyprland");
    }

    #[test]
    fn test_deserialize_invalid_string() {
        let yaml_str = r#"{ "preset": "invalid_preset" }"#;
        let config: serde_jsonrc::Value = serde_jsonrc::from_str(yaml_str).unwrap();
        let result = serde_jsonrc::from_value::<WorkspacePreset>(config["preset"].clone());
        assert!(result.is_err());
        println!("Expected error: {}", result.unwrap_err());
    }

    #[test]
    fn test_deserialize_invalid_object_type() {
        let yaml_str = r#"{ "preset": { "type": "invalid_preset" } }"#;
        let config: serde_jsonrc::Value = serde_jsonrc::from_str(yaml_str).unwrap();
        let result = serde_jsonrc::from_value::<WorkspacePreset>(config["preset"].clone());
        assert!(result.is_err());
        println!("Expected error: {}", result.unwrap_err());
    }
}
