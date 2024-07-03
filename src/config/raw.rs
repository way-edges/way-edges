use serde::Deserialize;
use serde_jsonrc::Value;

use super::NumOrRelative;

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct RawMargins {
    #[serde(default)]
    #[serde(deserialize_with = "super::transform_num_or_relative")]
    pub top: NumOrRelative,
    #[serde(default)]
    #[serde(deserialize_with = "super::transform_num_or_relative")]
    pub left: NumOrRelative,
    #[serde(default)]
    #[serde(deserialize_with = "super::transform_num_or_relative")]
    pub right: NumOrRelative,
    #[serde(default)]
    #[serde(deserialize_with = "super::transform_num_or_relative")]
    pub bottom: NumOrRelative,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct RawConfig {
    #[serde(default = "dt_edge")]
    pub edge: String,
    #[serde(default)]
    pub position: String,
    #[serde(default = "dt_layer")]
    pub layer: String,
    #[serde(default)]
    #[serde(deserialize_with = "super::transform_num_or_relative")]
    pub width: NumOrRelative,
    #[serde(default)]
    #[serde(deserialize_with = "super::transform_num_or_relative")]
    pub height: NumOrRelative,
    #[serde(default)]
    pub monitor_id: usize,
    #[serde(default)]
    pub monitor_name: String,
    #[serde(default)]
    pub margin: RawMargins,

    pub widget: Value,
}
fn dt_edge() -> String {
    String::from("left")
}
fn dt_layer() -> String {
    String::from("top")
}

#[derive(Deserialize, Debug)]
pub struct RawGroup {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub widgets: Vec<RawConfig>,
}
#[derive(Deserialize, Debug)]
pub struct RawTemp {
    #[serde(default)]
    pub groups: Vec<RawGroup>,
}
