use crate::shared::Curve;
use crate::widgets::common::{
    color_translate, dt_family_owned, schema_color, schema_optional_template, FamilyOwnedRef,
};
use cosmic_text::{Color, FamilyOwned};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer};
use util::color::parse_color;
use util::template::{
    arg::{TemplateArgFloatProcesser, TemplateArgRingPresetProcesser},
    base::{Template, TemplateProcesser},
};

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "lowercase", tag = "type")]
#[schemars(deny_unknown_fields)]
pub enum RingPreset {
    Ram {
        #[serde(default = "dt_update_interval")]
        update_interval: u64,
    },
    Swap {
        #[serde(default = "dt_update_interval")]
        update_interval: u64,
    },
    Cpu {
        #[serde(default = "dt_update_interval")]
        update_interval: u64,
        #[serde(default)]
        core: Option<usize>,
    },
    Battery {
        #[serde(default = "dt_update_interval")]
        update_interval: u64,
    },
    Disk {
        #[serde(default = "dt_update_interval")]
        update_interval: u64,
        #[serde(default = "dt_partition")]
        partition: String,
    },
    Custom {
        update_interval: u64,
        cmd: String,
    },
}
impl Default for RingPreset {
    fn default() -> Self {
        Self::Custom {
            update_interval: dt_update_interval(),
            cmd: String::default(),
        }
    }
}
fn dt_partition() -> String {
    "/".to_string()
}
fn dt_update_interval() -> u64 {
    1000
}

#[derive(Deserialize, Debug)]
pub struct RingConfigShadow {
    #[serde(default = "dt_r")]
    pub radius: i32,
    #[serde(default = "dt_rw")]
    pub ring_width: i32,
    #[serde(default = "dt_bg")]
    #[serde(deserialize_with = "color_translate")]
    pub bg_color: Color,
    #[serde(default = "dt_fg")]
    #[serde(deserialize_with = "color_translate")]
    pub fg_color: Color,

    #[serde(default = "dt_tt")]
    pub text_transition_ms: u64,
    #[serde(default)]
    pub animation_curve: Curve,

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

    #[serde(default = "dt_family_owned")]
    #[serde(with = "FamilyOwnedRef")]
    pub font_family: FamilyOwned,
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
fn dt_bg() -> Color {
    parse_color("#9F9F9F").unwrap()
}
fn dt_fg() -> Color {
    parse_color("#F1FA8C").unwrap()
}
fn dt_tt() -> u64 {
    300
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
            animation_curve: value.animation_curve,
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(from = "RingConfigShadow")]
#[schemars(deny_unknown_fields)]
pub struct RingConfig {
    pub radius: i32,
    pub ring_width: i32,
    #[schemars(schema_with = "schema_color")]
    pub bg_color: Color,
    #[schemars(schema_with = "schema_color")]
    pub fg_color: Color,

    pub text_transition_ms: u64,
    pub animation_curve: Curve,

    #[schemars(schema_with = "schema_optional_template")]
    pub prefix: Option<Template>,
    pub prefix_hide: bool,
    #[schemars(schema_with = "schema_optional_template")]
    pub suffix: Option<Template>,
    pub suffix_hide: bool,

    #[serde(default = "dt_family_owned")]
    #[serde(with = "FamilyOwnedRef")]
    pub font_family: FamilyOwned,
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
