use cosmic_text::Color;
use knus::Decode;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Deserializer;
use util::{
    color::{parse_color, COLOR_BLACK},
    template::{
        arg::TemplateArgFloatProcesser,
        base::{Template, TemplateProcesser},
    },
};

use crate::def::{
    shared::{
        color_translate, option_color_translate, schema_color, schema_optional_color,
        schema_optional_template, Curve, KeyEventMap,
    },
    util::{argv_str, parse_optional_color},
};

#[derive(Debug, Clone, JsonSchema, Deserialize)]
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
impl<S: knus::traits::ErrorSpan> knus::Decode<S> for Preset {
    fn decode_node(
        node: &knus::ast::SpannedNode<S>,
        ctx: &mut knus::decode::Context<S>,
    ) -> Result<Self, knus::errors::DecodeError<S>> {
        match argv_str(node, ctx)?.as_ref() {
            "speaker" => Ok(Self::Speaker(PulseAudioConfig::decode_node(node, ctx)?)),
            "microphone" => Ok(Self::Microphone(PulseAudioConfig::decode_node(node, ctx)?)),
            "backlight" => Ok(Self::Backlight(BacklightConfig::decode_node(node, ctx)?)),
            "custom" => Ok(Self::Custom(CustomConfig::decode_node(node, ctx)?)),
            name => Err(knus::errors::DecodeError::unexpected(
                node,
                "preset type",
                format!("unexpected preset type: {name}"),
            )),
        }
    }
}

#[derive(Debug, Decode, Clone, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct PulseAudioConfig {
    #[knus(
        child,
        default = default_mute_color(),
        unwrap(argument, decode_with = parse_color)
    )]
    #[serde(default = "default_mute_color", deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub mute_color: Color,
    #[knus(child, default,
        unwrap(argument, decode_with = parse_optional_color)
    )]
    #[serde(default, deserialize_with = "option_color_translate")]
    #[schemars(schema_with = "schema_optional_color")]
    pub mute_text_color: Option<Color>,

    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub animation_curve: Curve,

    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub device: Option<String>,
}

fn default_mute_color() -> Color {
    COLOR_BLACK
}

#[derive(Debug, Clone, Decode, Deserialize, JsonSchema, Default)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct BacklightConfig {
    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub device: Option<String>,
}

#[derive(Debug, Default, Clone, Decode, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct CustomConfig {
    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub update_command: String,
    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub update_interval: u64,

    #[knus(child, default, unwrap(argument, decode_with = slide_change_optional_template))]
    #[serde(default)]
    #[serde(deserialize_with = "slide_change_template")]
    #[schemars(schema_with = "schema_optional_template")]
    pub on_change_command: Option<Template>,

    #[knus(child, default)]
    #[serde(default)]
    pub event_map: KeyEventMap,
}

fn slide_change_optional_template(s: &str) -> Result<Option<Template>, String> {
    if s.is_empty() {
        Ok(None)
    } else {
        Template::create_from_str(
            s,
            TemplateProcesser::new().add_processer(TemplateArgFloatProcesser),
        )
        .map(Some)
    }
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
