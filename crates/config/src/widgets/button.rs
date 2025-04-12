use super::common::{self, CommonSize, KeyEventMap};
use cosmic_text::Color;
use educe::Educe;
use schemars::JsonSchema;
use serde::Deserialize;
use util::color::{parse_color, COLOR_BLACK};
use way_edges_derive::GetSize;

#[derive(Educe, Deserialize, GetSize, JsonSchema)]
#[educe(Debug)]
pub struct BtnConfig {
    #[serde(flatten)]
    pub size: CommonSize,
    #[serde(default = "dt_color")]
    #[serde(deserialize_with = "common::color_translate")]
    #[schemars(schema_with = "common::schema_color")]
    pub color: Color,
    #[serde(default = "dt_border_width")]
    pub border_width: i32,
    #[serde(default = "dt_border_color")]
    #[serde(deserialize_with = "common::color_translate")]
    #[schemars(schema_with = "common::schema_color")]
    pub border_color: Color,

    #[serde(default)]
    pub event_map: KeyEventMap,
}

fn dt_color() -> Color {
    parse_color("#7B98FF").unwrap()
}
fn dt_border_width() -> i32 {
    3
}
fn dt_border_color() -> Color {
    COLOR_BLACK
}
