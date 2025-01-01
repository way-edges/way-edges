use super::common::{self, CommonSize, KeyEventMap};
use educe::Educe;
use gtk::gdk::RGBA;
use serde::Deserialize;
use std::str::FromStr;
use way_edges_derive::GetSize;

#[derive(Educe, Deserialize, GetSize)]
#[educe(Debug)]
pub struct BtnConfig {
    #[serde(flatten)]
    pub size: CommonSize,
    #[serde(default = "dt_color")]
    #[serde(deserialize_with = "common::color_translate")]
    pub color: RGBA,
    #[serde(default = "dt_border_width")]
    pub border_width: i32,
    #[serde(default = "dt_border_color")]
    #[serde(deserialize_with = "common::color_translate")]
    pub border_color: RGBA,

    #[serde(default)]
    pub event_map: KeyEventMap,
}

fn dt_color() -> RGBA {
    RGBA::from_str("#7B98FF").unwrap()
}
fn dt_border_width() -> i32 {
    3
}
fn dt_border_color() -> RGBA {
    RGBA::BLACK
}
