use gtk4_layer_shell::Edge;

use crate::ui::EventMap;

pub struct Config {
    pub edge: Edge,
    pub position: Option<Edge>,
    pub size: (f64, f64),
    pub event_map: EventMap,
}
