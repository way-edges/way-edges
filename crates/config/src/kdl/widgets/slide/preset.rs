use cosmic_text::Color;
use knus::Decode;
use util::{
    color::{parse_color, COLOR_BLACK},
    template::{
        arg::TemplateArgFloatProcesser,
        base::{Template, TemplateProcesser},
    },
};

use crate::kdl::{
    shared::{Curve, KeyEventMap},
    util::{argv_str, parse_optional_color},
};

#[derive(Debug, Clone)]
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

#[derive(Debug, Decode, Clone)]
pub struct PulseAudioConfig {
    #[knus(
        child,
        default = default_mute_color(),
        unwrap(argument, decode_with = parse_color)
    )]
    pub mute_color: Color,
    #[knus(child, default,
        unwrap(argument, decode_with = parse_optional_color)
    )]
    pub mute_text_color: Option<Color>,

    #[knus(child, default, unwrap(argument))]
    pub animation_curve: Curve,

    #[knus(child, default, unwrap(argument))]
    pub device: Option<String>,
}

fn default_mute_color() -> Color {
    COLOR_BLACK
}

#[derive(Debug, Clone, Decode)]
pub struct BacklightConfig {
    #[knus(child, default, unwrap(argument))]
    pub device: Option<String>,
}

#[derive(Debug, Default, Clone, Decode)]
pub struct CustomConfig {
    #[knus(child, default, unwrap(argument))]
    pub update_command: String,
    #[knus(child, default, unwrap(argument))]
    pub update_interval: u64,

    #[knus(child, default, unwrap(argument, decode_with = slide_change_optional_template))]
    pub on_change_command: Option<Template>,

    #[knus(child, default)]
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
