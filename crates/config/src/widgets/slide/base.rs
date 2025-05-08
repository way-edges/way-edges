use cosmic_text::Color;
use educe::Educe;
use schemars::JsonSchema;
use serde::Deserialize;
use util::color::parse_color;
use way_edges_derive::{const_property, GetSize};

use super::{
    super::common::{self, CommonSize},
    preset::Preset,
};

use schemars::Schema;
use serde_json::Value;

// TODO: serde_valid
#[derive(Educe, Deserialize, GetSize, JsonSchema)]
#[educe(Debug)]
#[schemars(transform = SlideConfig_generate_defs)]
#[const_property("type", "slide")]
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
    #[serde(deserialize_with = "common::color_translate")]
    #[schemars(schema_with = "common::schema_color")]
    pub bg_color: Color,
    #[serde(default = "dt_fg_color")]
    #[serde(deserialize_with = "common::color_translate")]
    #[schemars(schema_with = "common::schema_color")]
    pub fg_color: Color,
    #[serde(default = "dt_border_color")]
    #[serde(deserialize_with = "common::color_translate")]
    #[schemars(schema_with = "common::schema_color")]
    pub border_color: Color,
    #[serde(default)]
    #[serde(deserialize_with = "common::option_color_translate")]
    #[schemars(schema_with = "common::schema_optional_color")]
    pub fg_text_color: Option<Color>,
    #[serde(default)]
    #[serde(deserialize_with = "common::option_color_translate")]
    #[schemars(schema_with = "common::schema_optional_color")]
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
