use std::str::FromStr;

use serde::Deserialize;
use serde_jsonrc::Value;

use super::NumOrRelative;

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct RawMargins {
    #[serde(default)]
    pub top: NumOrRelative,
    #[serde(default)]
    pub left: NumOrRelative,
    #[serde(default)]
    pub right: NumOrRelative,
    #[serde(default)]
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
    pub monitor_id: usize,
    #[serde(default)]
    pub monitor_name: String,
    #[serde(default)]
    pub margin: RawMargins,

    #[serde(default)]
    pub name: String,
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
pub struct RawRoot {
    #[serde(default)]
    pub groups: Vec<RawGroup>,
}

impl FromStr for RawRoot {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_jsonrc::from_str(s).map_err(|e| format!("JSON parse error: {e}"))
    }
}
