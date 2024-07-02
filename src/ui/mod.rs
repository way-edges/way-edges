mod draw_area;
mod draws;
mod window;

use std::collections::HashMap;

pub use window::*;
pub type EventMap = HashMap<u32, Box<dyn FnMut() + Send + Sync>>;
