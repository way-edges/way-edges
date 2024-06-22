use gtk::gdk::RGBA;
use gtk4_layer_shell::Edge;

use crate::ui::EventMap;

pub struct Config {
    pub edge: Edge,
    pub position: Option<Edge>,
    pub size: (f64, f64),
    pub event_map: EventMap,
    // pub color: RGBA,
    // transition_duration: u32,
    // frame_rate: u32,
    // extra_trigger_size: f64,
}

use serde::{Deserialize, Serialize};
// #[derive(Serialize, Deserialize)]
#[derive(Deserialize, Debug, Serialize)]
pub struct RawConfig {
    #[serde(default = "dt_edge")]
    pub edge: String,
    #[serde(default)]
    pub position: String,
    #[serde(default = "dt_width")]
    pub width: f64,
    #[serde(default)]
    pub height: f64,
    #[serde(default = "dt_rel_height")]
    pub rel_height: f64,
    #[serde(default)]
    pub event_map: Vec<(u32, String)>,
    #[serde(default = "dt_color")]
    pub color: String,
    #[serde(default = "dt_duration")]
    pub transition_duration: u32,
    #[serde(default = "dt_frame_rate")]
    pub frame_rate: u32,
    #[serde(default = "dt_trigger_size")]
    pub extra_trigger_size: f64,
}
fn dt_edge() -> String {
    String::from("left")
}
fn dt_width() -> f64 {
    15.
}
fn dt_rel_height() -> f64 {
    0.3
}
fn dt_color() -> String {
    String::from("#7B98FF")
}
fn dt_duration() -> u32 {
    300
}
fn dt_frame_rate() -> u32 {
    30
}
fn dt_trigger_size() -> f64 {
    5.
}

pub fn parse_config_test() {
    let data = r#"
        {
            "edge": "top",
            "position": "left",
            "width": 20,
            "rel_height": 0.5
        }"#;

    let res: RawConfig = serde_json::from_str(data).unwrap();
    println!("res: {:#?}", res);
}
