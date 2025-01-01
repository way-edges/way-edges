use educe::Educe;
use gtk::gdk::RGBA;
use serde::Deserialize;

use crate::widgets::common::{self};

#[derive(Educe, Deserialize)]
#[educe(Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum TextPreset {
    Time {
        #[serde(default = "dt_time_format")]
        format: String,
        #[serde(default)]
        time_zone: Option<String>,
    },
    Custom {
        update_with_interval_ms: (u64, String),
    },
}
fn dt_time_format() -> String {
    "%Y-%m-%d %H:%M:%S".to_string()
}

#[derive(Educe, Deserialize)]
#[educe(Debug)]
pub struct TextConfig {
    #[serde(default = "dt_fg_color")]
    #[serde(deserialize_with = "common::color_translate")]
    pub fg_color: RGBA,
    #[serde(default = "dt_font_size")]
    pub font_size: i32,
    #[serde(default)]
    pub font_family: Option<String>,

    pub preset: TextPreset,
}

fn dt_fg_color() -> RGBA {
    RGBA::BLACK
}
fn dt_font_size() -> i32 {
    24
}
