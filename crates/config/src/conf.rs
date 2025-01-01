use gtk4_layer_shell::{Edge, Layer};
use serde::Deserialize;
use std::collections::HashMap;

use crate::widgets::Widget;

use super::common::{
    deserialize_edge, deserialize_layer, deserialize_margins, deserialize_optional_edge,
    NumOrRelative,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum MonitorSpecifier {
    ID(usize),
    Name(String),
}
impl Default for MonitorSpecifier {
    fn default() -> Self {
        Self::ID(0)
    }
}

#[derive(Debug, Deserialize)]
struct ConfigShadow {
    #[serde(default = "dt_edge")]
    #[serde(deserialize_with = "deserialize_edge")]
    pub edge: Edge,

    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_edge")]
    pub position: Option<Edge>,

    #[serde(default = "dt_layer")]
    #[serde(deserialize_with = "deserialize_layer")]
    pub layer: Layer,

    #[serde(default)]
    #[serde(deserialize_with = "deserialize_margins")]
    pub margins: HashMap<Edge, NumOrRelative>,

    #[serde(default)]
    pub monitor: MonitorSpecifier,

    #[serde(default = "dt_name")]
    pub name: String,

    #[serde(default)]
    pub ignore_exclusive: bool,

    #[serde(default = "dt_transition_duration")]
    pub transition_duration: u64,
    #[serde(default)]
    pub frame_rate: Option<i32>,
    #[serde(default = "dt_extra_trigger_size")]
    pub extra_trigger_size: NumOrRelative,

    #[serde(default = "dt_preview_size")]
    pub preview_size: NumOrRelative,

    pub widget: Widget,
}

impl From<ConfigShadow> for Config {
    fn from(value: ConfigShadow) -> Self {
        let position;
        if let Some(pos) = value.position {
            position = pos
        } else {
            position = value.edge
        }
        Self {
            edge: value.edge,
            position,
            layer: value.layer,
            margins: value.margins,
            monitor: value.monitor,
            name: value.name,
            widget: Some(value.widget),
            ignore_exclusive: value.ignore_exclusive,
            transition_duration: value.transition_duration,
            frame_rate: value.frame_rate,
            extra_trigger_size: value.extra_trigger_size,
            preview_size: value.preview_size,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(from = "ConfigShadow")]
pub struct Config {
    pub edge: Edge,
    pub position: Edge,
    pub layer: Layer,
    pub margins: HashMap<Edge, NumOrRelative>,
    pub monitor: MonitorSpecifier,
    pub name: String,
    pub ignore_exclusive: bool,
    pub transition_duration: u64,
    pub frame_rate: Option<i32>,
    pub extra_trigger_size: NumOrRelative,
    pub preview_size: NumOrRelative,
    pub widget: Option<Widget>,
}

fn dt_name() -> String {
    gtk::gio::dbus_generate_guid().to_string()
}
fn dt_edge() -> Edge {
    Edge::Left
}
fn dt_layer() -> Layer {
    Layer::Top
}
fn dt_transition_duration() -> u64 {
    100
}
fn dt_extra_trigger_size() -> NumOrRelative {
    NumOrRelative::Num(1.0)
}
fn dt_preview_size() -> NumOrRelative {
    NumOrRelative::Num(0.0)
}
