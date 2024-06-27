use serde::Deserialize;

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct RawMargins {
    #[serde(default)]
    pub top: i32,
    #[serde(default)]
    pub left: i32,
    #[serde(default)]
    pub right: i32,
    #[serde(default)]
    pub bottom: i32,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct RawConfig {
    #[serde(default = "dt_edge")]
    pub edge: String,
    #[serde(default)]
    pub position: String,
    #[serde(default = "dt_layer")]
    pub layer: String,
    #[serde(default)]
    pub width: f64,
    #[serde(default)]
    pub height: f64,
    #[serde(default)]
    pub rel_height: f64,
    #[serde(default)]
    pub event_map: Vec<(u32, String)>,
    #[serde(default = "dt_color")]
    pub color: String,
    #[serde(default = "dt_duration")]
    pub transition_duration: u64,
    #[serde(default = "dt_frame_rate")]
    pub frame_rate: u64,
    #[serde(default = "dt_trigger_size")]
    pub extra_trigger_size: f64,
    #[serde(default)]
    pub monitor_id: usize,
    #[serde(default)]
    pub monitor_name: String,
    #[serde(default)]
    pub margin: RawMargins,
}
fn dt_edge() -> String {
    String::from("left")
}
fn dt_layer() -> String {
    String::from("top")
}
fn dt_color() -> String {
    String::from("#7B98FF")
}
fn dt_duration() -> u64 {
    300
}
fn dt_frame_rate() -> u64 {
    30
}
fn dt_trigger_size() -> f64 {
    5.
}

#[derive(Deserialize, Debug)]
pub struct RawGroup {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub widgets: Vec<RawConfig>,
}
#[derive(Deserialize, Debug)]
pub struct RawTemp {
    #[serde(default)]
    pub groups: Vec<RawGroup>,
}
