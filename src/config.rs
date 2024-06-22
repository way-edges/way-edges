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

pub struct RawConfig {
    pub edge: String,
    pub position: String,
    pub width: f64,
    pub height: f64,
    pub event_map: Vec<(u32, String)>,
    pub color: String,
    pub transition_duration: u32,
    pub frame_rate: u32,
    pub extra_trigger_size: f64,
}
