use knus::{errors::DecodeError, Decode};
use schemars::{json_schema, JsonSchema};
use serde::{Deserialize, Deserializer};
use serde_jsonrc::Value;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer};
use std::collections::HashSet;
use std::ops::Deref;

use crate::def::shared::{Curve, NumOrRelative};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MonitorSpecifier {
    Lists {
        ids: HashSet<usize>,
        names: HashSet<String>,
    },
    All,
}
impl Default for MonitorSpecifier {
    fn default() -> Self {
        Self::Lists {
            ids: HashSet::from([0]),
            names: HashSet::new(),
        }
    }
}
impl<S: knus::traits::ErrorSpan> knus::Decode<S> for MonitorSpecifier {
    fn decode_node(
        node: &knus::ast::SpannedNode<S>,
        ctx: &mut knus::decode::Context<S>,
    ) -> Result<Self, DecodeError<S>> {
        // check empty
        if node.arguments.is_empty() {
            return Err(DecodeError::unexpected(
                &node.node_name,
                "index or name",
                "MonitorSpecifier should have at least one argument",
            ));
        }
        // knus::Decode::D

        #[allow(clippy::collapsible_if)]
        if node.arguments.len() == 1 {
            if let knus::ast::Literal::String(s) = node.arguments[0].literal.deref() {
                if s.deref() == "*" {
                    return Ok(MonitorSpecifier::All);
                }
            }
        }

        let mut ids = HashSet::new();
        let mut names = HashSet::new();

        for arg in &node.arguments {
            match arg.literal.deref() {
                knus::ast::Literal::String(s) => {
                    if s.deref() == "*" {
                        return Err(DecodeError::unsupported(
                            &arg.literal,
                            "You cannot use the wildcard character '*' in a list of monitors, it is only allowed as the sole argument to specify all monitors",
                        ));
                    }
                    names.insert(s.to_string());
                }
                knus::ast::Literal::Int(value) => {
                    if let Ok(id) = value.try_into() {
                        ids.insert(id);
                    } else {
                        return Err(DecodeError::unsupported(
                            &arg.literal,
                            "Invalid integer value encountered",
                        ));
                    }
                }
                _ => {
                    return Err(DecodeError::unsupported(
                        &arg.literal,
                        "Unsupported value, only numbers and strings are recognized",
                    ));
                }
            }
        }

        Ok(MonitorSpecifier::Lists { ids, names })
    }
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
                        "type": ["string", "number"],
                    },
                }
            ],
        })
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
                Ok(MonitorSpecifier::Lists {
                    ids: HashSet::from([value as usize]),
                    names: HashSet::new(),
                })
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if value == "*" {
                    Ok(MonitorSpecifier::All)
                } else {
                    Ok(MonitorSpecifier::Lists {
                        ids: HashSet::new(),
                        names: HashSet::from([value.to_string()]),
                    })
                }
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'ae>,
            {
                let mut ids = HashSet::new();
                let mut names = HashSet::new();
                while let Some(value) = seq.next_element::<Value>()? {
                    match value {
                        Value::String(s) => {
                            if s == "*" {
                                return Err(serde::de::Error::invalid_value(
                                    serde::de::Unexpected::Str(&s),
                                    &"You cannot use the wildcard character '*' in a list of monitors, it is only allowed as the sole argument to specify all monitors",
                                ));
                            }
                            names.insert(s);
                        }
                        Value::Number(num) => {
                            if let Some(id) = num.as_u64() {
                                ids.insert(id as usize);
                            } else {
                                return Err(serde::de::Error::invalid_value(
                                    serde::de::Unexpected::Other(&format!("number {}", num)),
                                    &"Invalid integer value encountered",
                                ));
                            }
                        }
                        _ => {
                            return Err(serde::de::Error::invalid_type(
                                serde::de::Unexpected::Other(&format!("{:?}", value)),
                                &"a string or a number",
                            ));
                        }
                    }
                }
                Ok(MonitorSpecifier::Lists { ids, names })
            }
        }

        deserializer.deserialize_any(MonitorSpecifierVisitor)
    }
}

#[derive(Debug, Clone, Default, Decode, Deserialize, JsonSchema)]
pub struct Margins {
    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub left: NumOrRelative,
    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub top: NumOrRelative,
    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub right: NumOrRelative,
    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub bottom: NumOrRelative,
}

#[derive(Debug, Clone, Decode, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[schemars(deny_unknown_fields)]
pub struct CommonConfig {
    #[knus(child, unwrap(argument, decode_with = match_edge))]
    #[serde(deserialize_with = "deserialize_edge")]
    #[schemars(schema_with = "schema_edge")]
    pub edge: Anchor,

    #[knus(child, unwrap(argument, decode_with = match_edge))]
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_edge")]
    #[schemars(schema_with = "schema_optional_edge")]
    pub position: Option<Anchor>,

    #[knus(child, default=dt_layer(), unwrap(argument, decode_with = match_layer))]
    #[serde(default = "dt_layer")]
    #[serde(deserialize_with = "deserialize_layer")]
    #[schemars(schema_with = "schema_layer")]
    pub layer: Layer,

    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub offset: NumOrRelative,

    #[knus(child, default)]
    #[serde(default)]
    pub margins: Margins,

    #[knus(child, default)]
    #[serde(default)]
    pub monitor: MonitorSpecifier,

    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub namespace: String,

    #[knus(child)]
    #[serde(default)]
    pub ignore_exclusive: bool,

    #[knus(child, default = dt_transition_duration(), unwrap(argument))]
    #[serde(default = "dt_transition_duration")]
    pub transition_duration: u64,

    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub animation_curve: Curve,

    #[knus(child, default = dt_extra_trigger_size(), unwrap(argument))]
    #[serde(default = "dt_extra_trigger_size")]
    pub extra_trigger_size: NumOrRelative,

    #[knus(child, default = dt_preview_size(), unwrap(argument))]
    #[serde(default = "dt_preview_size")]
    pub preview_size: NumOrRelative,

    // TODO: true
    #[knus(child)]
    #[serde(default)]
    pub pinnable: bool,

    // TODO: true
    #[knus(child)]
    #[serde(default)]
    pub pin_with_key: bool,

    #[knus(child, default = dt_pin_key(), unwrap(argument))]
    #[serde(default = "dt_pin_key")]
    pub pin_key: u32,

    #[knus(child)]
    #[serde(default)]
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

        // offset & extra
        let max = match self.edge {
            Anchor::LEFT | Anchor::RIGHT => size.0,
            Anchor::TOP | Anchor::BOTTOM => size.1,
            _ => unreachable!(),
        };
        if self.offset.is_relative() {
            self.offset.calculate_relative(max as f64);
        }
        if self.extra_trigger_size.is_relative() {
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
fn dt_pin_key() -> u32 {
    smithay_client_toolkit::seat::pointer::BTN_MIDDLE
}

fn match_edge(edge: &str) -> Result<Anchor, std::io::Error> {
    Ok(match edge {
        "top" => Anchor::TOP,
        "left" => Anchor::LEFT,
        "bottom" => Anchor::BOTTOM,
        "right" => Anchor::RIGHT,
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid edge: {}", edge),
            ))
        }
    })
}

fn match_layer(layer: &str) -> Result<Layer, std::io::Error> {
    Ok(match layer {
        "background" => Layer::Background,
        "bottom" => Layer::Bottom,
        "top" => Layer::Top,
        "overlay" => Layer::Overlay,
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid layer: {}", layer),
            ))
        }
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
            Ok(Some(match_edge(v).map_err(|_| {
                serde::de::Error::invalid_value(serde::de::Unexpected::Str(v), &self)
            })?))
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
            match_layer(v)
                .map_err(|_| serde::de::Error::invalid_value(serde::de::Unexpected::Str(v), &self))
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
