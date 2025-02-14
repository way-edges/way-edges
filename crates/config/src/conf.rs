use serde::Deserialize;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer};

use crate::{common::Curve, widgets::Widget};

use super::common::{
    deserialize_edge, deserialize_layer, deserialize_optional_edge, NumOrRelative,
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

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Margins {
    #[serde(default)]
    pub left: NumOrRelative,
    #[serde(default)]
    pub top: NumOrRelative,
    #[serde(default)]
    pub right: NumOrRelative,
    #[serde(default)]
    pub bottom: NumOrRelative,
}

#[derive(Debug, Deserialize)]
struct ConfigShadow {
    #[serde(default = "dt_edge")]
    #[serde(deserialize_with = "deserialize_edge")]
    pub edge: Anchor,

    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_edge")]
    pub position: Option<Anchor>,

    #[serde(default = "dt_layer")]
    #[serde(deserialize_with = "deserialize_layer")]
    pub layer: Layer,

    #[serde(default)]
    pub margins: Margins,

    #[serde(default)]
    pub monitor: MonitorSpecifier,

    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub ignore_exclusive: bool,

    #[serde(default = "dt_transition_duration")]
    pub transition_duration: u64,
    #[serde(default)]
    pub animation_curve: Curve,
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
            extra_trigger_size: value.extra_trigger_size,
            preview_size: value.preview_size,
            animation_curve: value.animation_curve,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(from = "ConfigShadow")]
pub struct Config {
    pub edge: Anchor,
    pub position: Anchor,
    pub layer: Layer,
    pub margins: Margins,
    pub monitor: MonitorSpecifier,
    pub name: Option<String>,
    pub ignore_exclusive: bool,
    pub transition_duration: u64,
    pub animation_curve: Curve,
    pub extra_trigger_size: NumOrRelative,
    pub preview_size: NumOrRelative,
    pub widget: Option<Widget>,
}
impl Config {
    pub fn resolve_relative(&mut self, size: (i32, i32)) {
        // margins
        macro_rules! calculate_margins {
            ($m:expr, $s:expr) => {
                if $m.is_relative() {
                    $m.calculate_relative($s as f64);
                }
            };
        }
        calculate_margins!(self.margins.left, size.0);
        calculate_margins!(self.margins.right, size.0);
        calculate_margins!(self.margins.top, size.1);
        calculate_margins!(self.margins.bottom, size.1);

        // extra
        if self.extra_trigger_size.is_relative() {
            let max = match self.edge {
                Anchor::LEFT | Anchor::RIGHT => size.0,
                Anchor::TOP | Anchor::BOTTOM => size.1,
                _ => unreachable!(),
            };
            self.extra_trigger_size.calculate_relative(max as f64);
        }
    }
}

fn dt_edge() -> Anchor {
    Anchor::LEFT
}
fn dt_layer() -> Layer {
    Layer::Top
}
fn dt_transition_duration() -> u64 {
    300
}
fn dt_extra_trigger_size() -> NumOrRelative {
    NumOrRelative::Num(1.0)
}
fn dt_preview_size() -> NumOrRelative {
    NumOrRelative::Num(0.0)
}
