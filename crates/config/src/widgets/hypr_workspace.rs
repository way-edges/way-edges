use super::common::{self, from_value, CommonSize};
use super::Widget;
use educe::Educe;
use gtk::gdk::RGBA;
use serde::Deserialize;
use serde_jsonrc::Value;
use std::str::FromStr;
use way_edges_derive::GetSize;

pub const NAME: &str = "hyprland-workspace";

#[derive(Educe, Deserialize, GetSize)]
#[educe(Debug)]
pub struct HyprWorkspaceConfig {
    #[serde(flatten)]
    // flatten does not support `default` yet.
    // issue: https://github.com/serde-rs/serde/issues/1626
    // PR: https://github.com/serde-rs/serde/pull/2687
    // #[serde(default = "dt_size")]
    pub size: CommonSize,

    #[serde(default = "dt_gap")]
    pub gap: i32,
    #[serde(default = "dt_active_increase")]
    pub active_increase: f64,

    #[serde(default = "dt_workspace_transition_duration")]
    pub workspace_transition_duration: u64,

    #[serde(default = "dt_deactive_color")]
    #[serde(deserialize_with = "common::color_translate")]
    pub deactive_color: RGBA,
    #[serde(default = "dt_active_color")]
    #[serde(deserialize_with = "common::color_translate")]
    pub active_color: RGBA,
    #[serde(default)]
    #[serde(deserialize_with = "common::option_color_translate")]
    pub hover_color: Option<RGBA>,
}

fn dt_gap() -> i32 {
    5
}
fn dt_active_increase() -> f64 {
    0.5
}
fn dt_workspace_transition_duration() -> u64 {
    100
}

fn dt_deactive_color() -> RGBA {
    RGBA::from_str("#003049").unwrap()
}
fn dt_active_color() -> RGBA {
    RGBA::from_str("#669bbc").unwrap()
}

pub fn visit_config(d: Value) -> Result<Widget, String> {
    let c = from_value::<HyprWorkspaceConfig>(d)?;
    Ok(Widget::HyprWorkspace(Box::new(c)))
}
