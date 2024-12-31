use std::str::FromStr;

use crate::widgets::common::color_translate;
use gtk::gdk::RGBA;
use serde::{Deserialize, Deserializer};
use util::template::{
    arg::{TemplateArgFloatProcesser, TemplateArgRingPresetProcesser},
    base::{Template, TemplateProcesser},
};

#[derive(Debug, Deserialize)]
pub enum RingPreset {
    Ram,
    Swap,
    Cpu,
    Battery,
    Disk { partition: String },
    Custom { interval_update: (u64, String) },
}
impl Default for RingPreset {
    fn default() -> Self {
        Self::Custom {
            interval_update: (Default::default(), Default::default()),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct RingConfigShadow {
    #[serde(default = "dt_r")]
    pub radius: i32,
    #[serde(default = "dt_rw")]
    pub ring_width: i32,
    #[serde(default = "dt_bg")]
    #[serde(deserialize_with = "color_translate")]
    pub bg_color: RGBA,
    #[serde(default = "dt_fg")]
    #[serde(deserialize_with = "color_translate")]
    pub fg_color: RGBA,

    #[serde(default = "dt_tt")]
    pub text_transition_ms: u64,

    #[serde(default)]
    #[serde(deserialize_with = "ring_text_template")]
    pub prefix: Option<Template>,
    #[serde(default)]
    pub prefix_hide: bool,
    #[serde(default)]
    #[serde(deserialize_with = "ring_text_template")]
    pub suffix: Option<Template>,
    #[serde(default)]
    pub suffix_hide: bool,

    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub font_size: Option<i32>,

    pub preset: RingPreset,
}

fn dt_r() -> i32 {
    13
}
fn dt_rw() -> i32 {
    5
}
fn dt_bg() -> RGBA {
    RGBA::from_str("#9F9F9F").unwrap()
}
fn dt_fg() -> RGBA {
    RGBA::from_str("#F1FA8C").unwrap()
}
fn dt_tt() -> u64 {
    100
}

impl From<RingConfigShadow> for RingConfig {
    fn from(value: RingConfigShadow) -> Self {
        let font_size = value.font_size.unwrap_or(value.radius * 2);
        Self {
            radius: value.radius,
            ring_width: value.ring_width,
            bg_color: value.bg_color,
            fg_color: value.fg_color,
            text_transition_ms: value.text_transition_ms,
            prefix: value.prefix,
            prefix_hide: value.prefix_hide,
            suffix: value.suffix,
            suffix_hide: value.suffix_hide,
            font_family: value.font_family,
            font_size,
            preset: value.preset,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(from = "RingConfigShadow")]
pub struct RingConfig {
    pub radius: i32,
    pub ring_width: i32,
    pub bg_color: RGBA,
    pub fg_color: RGBA,

    pub text_transition_ms: u64,

    pub prefix: Option<Template>,
    pub prefix_hide: bool,
    pub suffix: Option<Template>,
    pub suffix_hide: bool,

    pub font_family: Option<String>,
    pub font_size: i32,

    pub preset: RingPreset,
}

pub fn ring_text_template<'de, D>(d: D) -> Result<Option<Template>, D::Error>
where
    D: Deserializer<'de>,
{
    struct EventMapVisitor;
    impl serde::de::Visitor<'_> for EventMapVisitor {
        type Value = Option<Template>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("vec of tuples: (key: number, command: string)")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            self.visit_string(v.to_string())
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(
                Template::create_from_str(
                    &v,
                    TemplateProcesser::new()
                        .add_processer(TemplateArgFloatProcesser)
                        .add_processer(TemplateArgRingPresetProcesser),
                )
                .map_err(serde::de::Error::custom)?,
            ))
        }
    }
    d.deserialize_any(EventMapVisitor)
}
