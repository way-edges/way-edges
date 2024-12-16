use super::common::{self, from_value, CommonSize, EventMap};
use crate::{NumOrRelative, Widget};
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

    #[educe(Debug(ignore))]
    #[serde(default = "common::dt_event_map")]
    #[serde(deserialize_with = "common::event_map_translate")]
    pub event_map: Option<EventMap>,

    #[serde(default = "dt_color")]
    #[serde(deserialize_with = "common::color_translate")]
    pub color: RGBA,
    #[serde(default = "common::dt_transition_duration")]
    pub transition_duration: u64,
    #[serde(default = "common::dt_frame_rate")]
    pub frame_rate: u32,
    #[serde(default = "common::dt_extra_trigger_size")]
    pub extra_trigger_size: NumOrRelative,
}

fn dt_color() -> RGBA {
    RGBA::from_str("#7B98FF").unwrap()
}

pub fn visit_config(d: Value) -> Result<Widget, String> {
    let c = from_value::<BtnConfig>(d)?;
    Ok(Widget::Btn(Box::new(c)))
}
