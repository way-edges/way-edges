use std::str::FromStr;

use crate::config::{widgets::slide::update_task_interval, Widget};

use super::{
    common::{color_translate, dt_frame_rate},
    slide::UpdateTask,
};
use educe::Educe;
use gtk::gdk::RGBA;
use serde::Deserialize;
use serde_jsonrc::Value;

pub const NAME: &str = "ring";

#[derive(Deserialize, Educe)]
#[educe(Debug)]
pub struct RingConfig {
    #[serde(default = "dt_r")]
    pub radius: f64,
    #[serde(default = "dt_rw")]
    pub ring_width: f64,
    #[serde(default = "dt_bg")]
    #[serde(deserialize_with = "color_translate")]
    pub bg_color: RGBA,
    #[serde(default = "dt_fg")]
    #[serde(deserialize_with = "color_translate")]
    pub fg_color: RGBA,

    #[serde(default = "dt_frame_rate")]
    pub frame_rate: u32,
    #[serde(default = "dt_tt")]
    pub text_transition_ms: u64,

    #[educe(Debug(ignore))]
    #[serde(deserialize_with = "update_task_interval")]
    pub update_with_interval_ms: Option<(u64, UpdateTask)>,
}

fn dt_r() -> f64 {
    5.0
}
fn dt_rw() -> f64 {
    13.0
}
fn dt_bg() -> RGBA {
    RGBA::from_str("#9F9F9F").unwrap()
}
fn dt_fg() -> RGBA {
    RGBA::from_str("#F1FA8C").unwrap()
}
fn dt_tt() -> u64 {
    100
}

pub fn visit_config(d: Value) -> Result<Widget, String> {
    let r = super::common::from_value::<RingConfig>(d)?;
    Ok(Widget::Ring(Box::new(r)))
}
