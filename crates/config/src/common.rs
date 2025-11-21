use schemars::json_schema;
use serde::Deserializer;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer};

use schemars::JsonSchema;
use std::collections::HashSet;

use serde::Deserialize;

use crate::shared::{Curve, NumOrRelative};

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
#[serde(rename_all = "kebab-case")]
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
#[serde(rename_all = "kebab-case")]
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
    offset: i32,

    #[serde(default)]
    pub margins: Margins,

    #[serde(default)]
    pub monitor: MonitorSpecifier,

    #[serde(default)]
    pub namespace: String,

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

    #[serde(default)]
    pub pin_on_startup: bool,
}

impl From<ConfigShadow> for CommonConfig {
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
            offset: value.offset,
            margins: value.margins,
            monitor: value.monitor,
            namespace: value.namespace,
            ignore_exclusive: value.ignore_exclusive,
            transition_duration: value.transition_duration,
            extra_trigger_size: value.extra_trigger_size,
            preview_size: value.preview_size,
            animation_curve: value.animation_curve,
            pinnable: value.pinnable,
            pin_with_key: value.pin_with_key,
            pin_key: value.pin_key,
            pin_on_startup: value.pin_on_startup,
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(from = "ConfigShadow")]
#[schemars(deny_unknown_fields, !from)]
#[serde(rename_all = "kebab-case")]
pub struct CommonConfig {
    #[schemars(schema_with = "schema_edge")]
    pub edge: Anchor,
    #[schemars(schema_with = "schema_optional_edge")]
    pub position: Anchor,
    #[schemars(schema_with = "schema_layer")]
    pub layer: Layer,
    pub offset: i32,
    pub margins: Margins,
    pub monitor: MonitorSpecifier,
    pub namespace: String,
    pub ignore_exclusive: bool,
    pub transition_duration: u64,
    pub animation_curve: Curve,
    pub extra_trigger_size: NumOrRelative,
    pub preview_size: NumOrRelative,

    pub pin_with_key: bool,
    pub pin_key: u32,
    pub pinnable: bool,
    pub pin_on_startup: bool,
}
impl CommonConfig {
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

fn match_edge(edge: &str) -> Option<Anchor> {
    Some(match edge {
        "top" => Anchor::TOP,
        "left" => Anchor::LEFT,
        "bottom" => Anchor::BOTTOM,
        "right" => Anchor::RIGHT,
        _ => return None,
    })
}

pub fn deserialize_optional_edge<'de, D>(d: D) -> Result<Option<Anchor>, D::Error>
where
    D: Deserializer<'de>,
{
    struct EventMapVisitor;
    impl serde::de::Visitor<'_> for EventMapVisitor {
        type Value = Option<Anchor>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("edge only support: left, right, top, bottom")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if let Some(edge) = match_edge(v) {
                Ok(Some(edge))
            } else {
                Err(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Str(v),
                    &self,
                ))
            }
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            self.visit_str(v.as_str())
        }
    }

    d.deserialize_any(EventMapVisitor)
}

pub fn deserialize_edge<'de, D>(d: D) -> Result<Anchor, D::Error>
where
    D: Deserializer<'de>,
{
    if let Some(edge) = deserialize_optional_edge(d)? {
        Ok(edge)
    } else {
        Err(serde::de::Error::missing_field("edge is not optional"))
    }
}

pub fn deserialize_layer<'de, D>(d: D) -> Result<Layer, D::Error>
where
    D: Deserializer<'de>,
{
    struct EventMapVisitor;
    impl serde::de::Visitor<'_> for EventMapVisitor {
        type Value = Layer;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("layer only support: background, bottom, top, overlay")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let edge = match v {
                "background" => Layer::Background,
                "bottom" => Layer::Bottom,
                "top" => Layer::Top,
                "overlay" => Layer::Overlay,
                _ => {
                    return Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Str(v),
                        &self,
                    ));
                }
            };
            Ok(edge)
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            self.visit_str(v.as_str())
        }
    }

    d.deserialize_any(EventMapVisitor)
}

pub fn schema_edge(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    json_schema!({
        "type": "string",
        "enum": ["top", "bottom", "left", "right"]
    })
}
pub fn schema_optional_edge(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    json_schema!({
        "type": ["string", "null"],
        "enum": ["top", "bottom", "left", "right"]
    })
}
pub fn schema_layer(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    json_schema!({
        "type": "string",
        "enum": ["top", "bottom", "background", "overlay"]
    })
}
