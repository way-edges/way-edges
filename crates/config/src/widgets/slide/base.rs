use cosmic_text::Color;
use educe::Educe;
use schemars::JsonSchema;
use serde::Deserialize;
use util::color::parse_color;
use way_edges_derive::{const_property, GetSize};

use crate::shared::{
    color_translate, option_color_translate, schema_color, schema_optional_color, CommonSize,
};

use super::preset::Preset;

use schemars::Schema;
use serde_json::Value;

// TODO: serde_valid
#[derive(Educe, Deserialize, GetSize, JsonSchema, Clone)]
#[educe(Debug)]
#[schemars(deny_unknown_fields)]
#[schemars(transform = SlideConfig_generate_defs)]
#[const_property("type", "slide")]
#[serde(rename_all = "kebab-case")]
pub struct SlideConfig {
    // draw related
    #[serde(flatten)]
    pub size: CommonSize,
    #[serde(default = "dt_border_width")]
    pub border_width: i32,

    #[serde(default = "dt_obtuse_angle")]
    pub obtuse_angle: f64,
    #[serde(default = "dt_radius")]
    pub radius: f64,

    #[serde(default = "dt_bg_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub bg_color: Color,
    #[serde(default = "dt_fg_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub fg_color: Color,
    #[serde(default = "dt_border_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub border_color: Color,
    #[serde(default)]
    #[serde(deserialize_with = "option_color_translate")]
    #[schemars(schema_with = "schema_optional_color")]
    pub fg_text_color: Option<Color>,
    #[serde(default)]
    #[serde(deserialize_with = "option_color_translate")]
    #[schemars(schema_with = "schema_optional_color")]
    pub bg_text_color: Option<Color>,

    #[serde(default)]
    pub redraw_only_on_internal_update: bool,

    #[serde(default)]
    pub preset: Preset,
}

fn dt_border_width() -> i32 {
    3
}
fn dt_bg_color() -> Color {
    parse_color("#808080").unwrap()
}
fn dt_fg_color() -> Color {
    parse_color("#FFB847").unwrap()
}
fn dt_border_color() -> Color {
    parse_color("#646464").unwrap()
}
fn dt_obtuse_angle() -> f64 {
    120.
}
fn dt_radius() -> f64 {
    20.
}
