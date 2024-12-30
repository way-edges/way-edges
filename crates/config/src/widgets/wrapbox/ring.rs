use std::str::FromStr;

use super::common::Template;
use crate::widgets::common::color_translate;
use gtk::gdk::RGBA;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub enum RingPreset {
    Ram,
    Swap,
    Cpu,
    Battery,
    Disk { partition: String },
    Custom { interval_update: (u64, String) },
}

#[derive(Deserialize, Debug)]
pub struct RingConfigShadow {
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

    #[serde(default)]
    pub frame_rate: Option<i32>,
    #[serde(default = "dt_tt")]
    pub text_transition_ms: u64,

    #[serde(default)]
    pub prefix: Option<Template>,
    #[serde(default)]
    pub prefix_hide: bool,
    #[serde(default)]
    pub suffix: Option<Template>,
    #[serde(default)]
    pub suffix_hide: bool,

    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub font_size: Option<i32>,

    pub preset: RingPreset,
}

fn dt_r() -> f64 {
    13.0
}
fn dt_rw() -> f64 {
    5.0
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

impl From<RingConfigShadow> for RingConfig {
    fn from(value: RingConfigShadow) -> Self {
        let font_size = value.font_size.unwrap_or((value.radius * 2.) as i32);
        Self {
            radius: value.radius,
            ring_width: value.ring_width,
            bg_color: value.bg_color,
            fg_color: value.fg_color,
            frame_rate: value.frame_rate,
            text_transition_ms: value.text_transition_ms,
            prefix: value.prefix,
            prefix_hide: value.prefix_hide,
            suffix: value.suffix,
            suffix_hide: value.suffix_hide,
            font_family: value.font_family,
            font_size,
            preset: value.preset,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(from = "RingConfigShadow")]
pub struct RingConfig {
    pub radius: f64,
    pub ring_width: f64,
    pub bg_color: RGBA,
    pub fg_color: RGBA,

    pub frame_rate: Option<i32>,
    pub text_transition_ms: u64,

    pub prefix: Option<Template>,
    pub prefix_hide: bool,
    pub suffix: Option<Template>,
    pub suffix_hide: bool,

    pub font_family: Option<String>,
    pub font_size: i32,

    pub preset: RingPreset,
}
