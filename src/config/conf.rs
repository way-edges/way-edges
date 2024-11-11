use crate::activate::monitor::MonitorSpecifier;
use educe::Educe;
use gtk4_layer_shell::{Edge, Layer};
use serde::{Deserialize, Deserializer};
use std::{collections::HashMap, str::FromStr};

use super::widgets::{
    self, backlight::BLConfig, button::BtnConfig, hypr_workspace::HyprWorkspaceConfig,
    pulseaudio::PAConfig, ring::RingConfig, slide::SlideConfig, text::TextConfig,
    wrapbox::BoxConfig,
};

pub type GroupConfig = Vec<Config>;

#[derive(Debug, Clone, Copy)]
pub enum NumOrRelative {
    Num(f64),
    Relative(f64),
}
impl Default for NumOrRelative {
    fn default() -> Self {
        Self::Num(f64::default())
    }
}
impl<'de> Deserialize<'de> for NumOrRelative {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct F64OrRelativeVisitor;
        impl<'de> serde::de::Visitor<'de> for F64OrRelativeVisitor {
            type Value = NumOrRelative;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a number or a string")
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(NumOrRelative::Num(v as f64))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(NumOrRelative::Num(v as f64))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(NumOrRelative::Num(v))
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                // just `unwrap`, it's ok
                lazy_static::lazy_static! {
                    static ref re: regex::Regex = regex::Regex::new(r"^(\d+(\.\d+)?)%\s*(.*)$").unwrap();
                }

                if let Some(captures) = re.captures(v) {
                    let percentage_str = captures.get(1).map_or("", |m| m.as_str());
                    let percentage = f64::from_str(percentage_str).map_err(E::custom)?;

                    Ok(NumOrRelative::Relative(percentage * 0.01))
                } else {
                    Err(E::custom(
                        "Input does not match the expected format.".to_string(),
                    ))
                }
            }
        }
        d.deserialize_any(F64OrRelativeVisitor)
    }
}

#[allow(dead_code)]
impl NumOrRelative {
    pub fn get_num(&self) -> Result<f64, &str> {
        if let Self::Num(r) = self {
            Ok(*r)
        } else {
            Err("relative, not num")
        }
    }
    pub fn get_num_into(self) -> Result<f64, &'static str> {
        if let Self::Num(r) = self {
            Ok(r)
        } else {
            Err("relative, not num")
        }
    }
    pub fn is_valid_length(&self) -> bool {
        match self {
            NumOrRelative::Num(r) => *r > f64::default(),
            NumOrRelative::Relative(r) => *r > 0.,
        }
    }
    pub fn get_rel(&self) -> Result<f64, &'static str> {
        if let Self::Relative(r) = self {
            Ok(*r)
        } else {
            Err("num, not relative")
        }
    }
    pub fn get_rel_into(self) -> Result<f64, &'static str> {
        if let Self::Relative(r) = self {
            Ok(r)
        } else {
            Err("num, not relative")
        }
    }
    pub fn calculate_relative_into(self, max: f64) -> Self {
        if let Self::Relative(r) = self {
            Self::Num(r * max)
        } else {
            self
        }
    }
    pub fn calculate_relative(&mut self, max: f64) {
        if let Self::Relative(r) = self {
            *self = Self::Num(*r * max)
        }
    }
}

#[derive(Educe)]
#[educe(Debug)]
pub enum Widget {
    Btn(Box<BtnConfig>),
    Slider(Box<SlideConfig>),
    PulseAudio(Box<PAConfig>),
    Backlight(Box<BLConfig>),
    WrapBox(Box<BoxConfig>),
    Ring(Box<RingConfig>),
    Text(Box<TextConfig>),
    HyprWorkspace(Box<HyprWorkspaceConfig>),
}

impl<'de> Deserialize<'de> for Widget {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = serde_jsonrc::value::Value::deserialize(deserializer)?;

        if !raw.is_object() {
            return Err(serde::de::Error::custom("Widget must be object"));
        }
        let t = raw
            .get("type")
            .ok_or(serde::de::Error::missing_field("type"))?
            .as_str()
            .ok_or(serde::de::Error::custom("widget type must be string"))?;

        match t {
            widgets::button::NAME => widgets::button::visit_config(raw),
            widgets::slide::NAME => widgets::slide::visit_config(raw),
            widgets::pulseaudio::NAME_SOUCE | widgets::pulseaudio::NAME_SINK => {
                widgets::pulseaudio::visit_config(raw)
            }
            widgets::backlight::NAME => widgets::backlight::visit_config(raw),
            widgets::wrapbox::NAME => widgets::wrapbox::visit_config(raw),
            widgets::ring::NAME => widgets::ring::visit_config(raw),
            widgets::text::NAME => widgets::text::visit_config(raw),
            widgets::hypr_workspace::NAME => widgets::hypr_workspace::visit_config(raw),
            _ => Err(format!("unknown widget type: {t}")),
        }
        .map_err(serde::de::Error::custom)
    }
}

#[derive(Educe, Deserialize)]
#[educe(Debug)]
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
    #[serde(default)]
    pub name: String,
    pub widget: Option<Widget>,
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
            widget: value.widget,
        }
    }
}

#[derive(Educe, Deserialize)]
#[educe(Debug)]
#[serde(from = "ConfigShadow")]
pub struct Config {
    // #[serde(default = "dt_edge")]
    // #[serde(deserialize_with = "deserialize_edge")]
    pub edge: Edge,

    // #[serde(default)]
    // // #[serde(deserialize_with = "deserialize_optional_edge")]
    // #[serde(deserialize_with = "deserialize_edge")]
    pub position: Edge,

    // #[serde(default = "dt_layer")]
    // #[serde(deserialize_with = "deserialize_layer")]
    pub layer: Layer,

    // #[serde(default)]
    // #[serde(deserialize_with = "deserialize_margins")]
    pub margins: HashMap<Edge, NumOrRelative>,

    // #[serde(default)]
    pub monitor: MonitorSpecifier,
    // #[serde(default)]
    pub name: String,
    pub widget: Option<Widget>,
}

fn dt_edge() -> Edge {
    Edge::Left
}
fn dt_layer() -> Layer {
    Layer::Top
}

impl Drop for Config {
    fn drop(&mut self) {
        log::debug!("dropping config: {self:?}")
    }
}

fn match_edge(edge: &str) -> Option<Edge> {
    Some(match edge {
        "top" => Edge::Top,
        "left" => Edge::Left,
        "bottom" => Edge::Bottom,
        "right" => Edge::Right,
        _ => return None,
    })
}

fn deserialize_optional_edge<'de, D>(d: D) -> Result<Option<Edge>, D::Error>
where
    D: Deserializer<'de>,
{
    struct EventMapVisitor;
    impl<'de> serde::de::Visitor<'de> for EventMapVisitor {
        type Value = Option<Edge>;

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

fn deserialize_edge<'de, D>(d: D) -> Result<Edge, D::Error>
where
    D: Deserializer<'de>,
{
    if let Some(edge) = deserialize_optional_edge(d)? {
        Ok(edge)
    } else {
        Err(serde::de::Error::missing_field("edge is not optional"))
    }
}

fn deserialize_layer<'de, D>(d: D) -> Result<Layer, D::Error>
where
    D: Deserializer<'de>,
{
    struct EventMapVisitor;
    impl<'de> serde::de::Visitor<'de> for EventMapVisitor {
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

fn deserialize_margins<'de, D>(d: D) -> Result<HashMap<Edge, NumOrRelative>, D::Error>
where
    D: Deserializer<'de>,
{
    struct EventMapVisitor;
    impl<'de> serde::de::Visitor<'de> for EventMapVisitor {
        type Value = HashMap<Edge, NumOrRelative>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("margins for `left/right/top/bottom` only support: int or str")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut v = HashMap::new();
            while let Some((key, value)) = map.next_entry::<&str, _>()? {
                let Some(edge) = match_edge(key) else {
                    return Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Str(key),
                        &self,
                    ));
                };
                v.insert(edge, value);
            }
            Ok(v)
        }
    }

    d.deserialize_any(EventMapVisitor)
}

#[derive(Deserialize, Debug)]
pub struct Group {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub widgets: Vec<Config>,
}
#[derive(Deserialize, Debug)]
pub struct Root {
    #[serde(default)]
    pub groups: Vec<Group>,
}

impl Root {
    pub fn take_group(&mut self, name: &str) -> Option<Group> {
        let position = self.groups.iter().position(|g| g.name == name)?;
        Some(self.groups.swap_remove(position))
    }
    pub fn take_first(&mut self) -> Option<Group> {
        if !self.groups.is_empty() {
            Some(self.groups.swap_remove(0))
        } else {
            None
        }
    }
}

pub fn parse_config(data: &str) -> Result<Root, String> {
    serde_jsonrc::from_str(data).map_err(|e| format!("JSON parse error: {e}"))
}
