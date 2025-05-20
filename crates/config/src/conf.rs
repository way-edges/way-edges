use schemars::{json_schema, JsonSchema};
use std::collections::HashSet;

use serde::Deserialize;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer};

use crate::{common::Curve, widgets::Widget};

use super::common::{
    deserialize_edge, deserialize_layer, deserialize_optional_edge, schema_edge, schema_layer,
    schema_optional_edge, NumOrRelative,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MonitorSpecifier {
    ID(usize),
    Names(HashSet<String>),
    All,

    // this shall not be used for deserialization
    Name(String),
}
impl JsonSchema for MonitorSpecifier {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "MonitorSpecifier".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        json_schema!({
            "oneOf": [
                {
                    "type": "string",
                },
                {
                    "enum": ["*"],
                },
                {
                    "type": "number",
                    "minimum": 0,
                },
                {
                    "type": "array",
                    "items": {
                        "type": "string",
                    },
                }
            ],
        })
    }
}
impl Default for MonitorSpecifier {
    fn default() -> Self {
        Self::ID(0)
    }
}
impl<'de> Deserialize<'de> for MonitorSpecifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct MonitorSpecifierVisitor;
        impl<'ae> serde::de::Visitor<'ae> for MonitorSpecifierVisitor {
            type Value = MonitorSpecifier;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a monitor ID or a list of monitor names")
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(MonitorSpecifier::ID(value as usize))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if value == "*" {
                    Ok(MonitorSpecifier::All)
                } else {
                    let mut hashset = HashSet::new();
                    hashset.insert(value.to_string());
                    Ok(MonitorSpecifier::Names(hashset))
                }
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'ae>,
            {
                let mut names = HashSet::new();
                while let Some(value) = seq.next_element::<String>()? {
                    names.insert(value);
                }
                Ok(MonitorSpecifier::Names(names))
            }
        }

        deserializer.deserialize_any(MonitorSpecifierVisitor)
    }
}

mod tests {

    #[test]
    fn test_monitor_specifier() {
        use super::*;
        use serde_jsonrc::json;

        #[derive(Debug, Deserialize)]
        struct TestConfig {
            monitor: MonitorSpecifier,
        }

        let json_data = json!({
            "monitor": 1,
        });
        let config: TestConfig = serde_jsonrc::from_value(json_data).unwrap();
        assert_eq!(config.monitor, MonitorSpecifier::ID(1));

        let json_data = json!({
            "monitor": "*",
        });
        let config: TestConfig = serde_jsonrc::from_value(json_data).unwrap();
        assert_eq!(config.monitor, MonitorSpecifier::All);

        let json_data = json!({
            "monitor": ["Monitor1", "Monitor2"],
        });
        let config: TestConfig = serde_jsonrc::from_value(json_data).unwrap();
        assert_eq!(
            config.monitor,
            MonitorSpecifier::Names(HashSet::from_iter(vec![
                "Monitor1".to_string(),
                "Monitor2".to_string()
            ]))
        );
    }
}

#[derive(Debug, Deserialize, Clone, Default, JsonSchema)]
#[schemars(deny_unknown_fields)]
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
pub(crate) struct ConfigShadow {
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

    #[serde(default = "dt_pinnable")]
    pub pinnable: bool,
    #[serde(default = "dt_pin_with_key")]
    pub pin_with_key: bool,
    #[serde(default = "dt_pin_key")]
    pub pin_key: u32,

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
            pinnable: value.pinnable,
            pin_with_key: value.pin_with_key,
            pin_key: value.pin_key,
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(from = "ConfigShadow")]
#[schemars(deny_unknown_fields)]
pub struct Config {
    #[schemars(schema_with = "schema_edge")]
    pub edge: Anchor,
    #[schemars(schema_with = "schema_optional_edge")]
    pub position: Anchor,
    #[schemars(schema_with = "schema_layer")]
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

    pub pin_with_key: bool,
    pub pin_key: u32,
    pub pinnable: bool,
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
fn dt_pinnable() -> bool {
    true
}
fn dt_pin_with_key() -> bool {
    true
}
fn dt_pin_key() -> u32 {
    smithay_client_toolkit::seat::pointer::BTN_MIDDLE
}
