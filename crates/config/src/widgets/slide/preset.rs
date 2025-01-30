use cosmic_text::Color;
use serde::{Deserialize, Deserializer};
use util::{
    color::COLOR_BLACK,
    template::{
        arg::TemplateArgFloatProcesser,
        base::{Template, TemplateProcesser},
    },
};

use crate::widgets::common::KeyEventMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
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

#[derive(Debug, Deserialize)]
pub struct PulseAudioConfig {
    #[serde(default = "default_mute_color")]
    #[serde(deserialize_with = "super::super::common::color_translate")]
    pub mute_color: Color,
    pub device: Option<String>,
}

fn default_mute_color() -> Color {
    COLOR_BLACK
}

#[derive(Debug, Deserialize)]
pub struct BacklightConfig {
    #[serde(default)]
    pub device: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CustomConfig {
    #[serde(default)]
    pub interval_update: (u64, String),

    #[serde(default)]
    #[serde(deserialize_with = "slide_change_template")]
    pub on_change: Option<Template>,

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
