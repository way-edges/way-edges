use std::str::FromStr;

use educe::Educe;
use gtk::gdk::RGBA;
use serde::Deserialize;
use way_edges_derive::GetSize;

use super::{
    super::common::{self, CommonSize},
    preset::Preset,
};

// TODO: serde_valid
#[derive(Educe, Deserialize, GetSize)]
#[educe(Debug)]
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
    pub bg_color: RGBA,
    #[serde(default = "dt_fg_color")]
    #[serde(deserialize_with = "common::color_translate")]
    pub fg_color: RGBA,
    #[serde(default = "dt_border_color")]
    #[serde(deserialize_with = "common::color_translate")]
    pub border_color: RGBA,
    #[serde(default = "dt_text_color")]
    #[serde(deserialize_with = "common::color_translate")]
    pub text_color: RGBA,

    #[serde(default)]
    pub redraw_only_on_internal_update: bool,

    #[serde(default)]
    pub preset: Preset,
}

fn dt_border_width() -> i32 {
    3
}
fn dt_bg_color() -> RGBA {
    RGBA::from_str("#808080").unwrap()
}
fn dt_fg_color() -> RGBA {
    RGBA::from_str("#FFB847").unwrap()
}
fn dt_border_color() -> RGBA {
    RGBA::from_str("#646464").unwrap()
}
fn dt_text_color() -> RGBA {
    RGBA::from_str("#000000").unwrap()
}
fn dt_obtuse_angle() -> f64 {
    120.
}
fn dt_radius() -> f64 {
    20.
}
