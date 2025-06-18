use cosmic_text::Color;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer};
use util::{
    color::COLOR_BLACK,
    template::{
        arg::TemplateArgFloatProcesser,
        base::{Template, TemplateProcesser},
    },
};

use crate::shared::{
    color_translate, option_color_translate, schema_color, schema_optional_color,
    schema_optional_template, Curve, KeyEventMap,
};

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum Preset {
    Speaker(PulseAudioConfig),
    Microphone(PulseAudioConfig),
    Backlight(BacklightConfig),
    Custom(CustomConfig),
}
impl Default for Preset {
    fn default() -> Self {
        Self::Custom(CustomConfig::default())
    }
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct PulseAudioConfig {
    #[serde(default = "default_mute_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub mute_color: Color,
    #[serde(default)]
    #[serde(deserialize_with = "option_color_translate")]
    #[schemars(schema_with = "schema_optional_color")]
    pub mute_text_color: Option<Color>,

    #[serde(default)]
    pub animation_curve: Curve,
    pub device: Option<String>,
}

fn default_mute_color() -> Color {
    COLOR_BLACK
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct BacklightConfig {
    #[serde(default)]
    pub device: Option<String>,
}

#[derive(Debug, Deserialize, Default, JsonSchema, Clone)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct CustomConfig {
    #[serde(default)]
    pub update_command: String,
    #[serde(default)]
    pub update_interval: u64,

    #[serde(default)]
    #[serde(deserialize_with = "slide_change_template")]
    #[schemars(schema_with = "schema_optional_template")]
    pub on_change_command: Option<Template>,

    #[serde(default)]
    pub event_map: KeyEventMap,
}

pub fn slide_change_template<'de, D>(d: D) -> Result<Option<Template>, D::Error>
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
                    TemplateProcesser::new().add_processer(TemplateArgFloatProcesser),
                )
                .map_err(serde::de::Error::custom)?,
            ))
        }
    }
    d.deserialize_any(EventMapVisitor)
}
