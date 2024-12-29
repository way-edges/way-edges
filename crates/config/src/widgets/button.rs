use super::common::{self, from_value, CommonSize, KeyEventMap};
use super::Widget;
use educe::Educe;
use gtk::gdk::RGBA;
use serde::Deserialize;
use serde_jsonrc::Value;
use std::str::FromStr;
use way_edges_derive::GetSize;

pub const NAME: &str = "btn";

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
pub fn visit_config(d: Value) -> Result<Widget, String> {
    let c = from_value::<BtnConfig>(d)?;
    Ok(Widget::Btn(Box::new(c)))
}
