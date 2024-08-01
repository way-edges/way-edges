use std::str::FromStr;

use crate::{
    config::{widgets::slide::update_task_interval, Widget},
    plug::system::{init_mem_info, init_system_info, register_disk_partition},
};

use super::{
    common::{color_translate, dt_frame_rate},
    slide::UpdateTask,
};
use educe::Educe;
use gtk::gdk::RGBA;
use serde::Deserialize;
use serde_jsonrc::Value;

pub const NAME: &str = "ring";

#[derive(Debug)]
pub enum RingPreset {
    Ram,
    Swap,
    Cpu,
    Battery,
    Disk(String),
    Custom(RingCustom),
}

#[derive(Deserialize, Educe)]
#[educe(Debug)]
pub struct RingCustom {
    #[educe(Debug(ignore))]
    #[serde(default)]
    #[serde(deserialize_with = "update_task_interval")]
    pub update_with_interval_ms: Option<(u64, UpdateTask)>,

    #[serde(default)]
    pub template: Option<String>,
}

#[derive(Deserialize, Educe)]
#[educe(Debug)]
pub struct RingCommon {
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

    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub font_family: Option<String>,
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

#[derive(Debug)]
pub struct RingConfig {
    pub common: RingCommon,
    pub preset: RingPreset,
}

pub fn visit_config(d: Value) -> Result<Widget, String> {
    let preset = {
        let preset = d.get("preset").ok_or("Ring preset not provided")?;
        let preset_type = {
            if let Some(s) = preset.as_str() {
                s
            } else {
                preset
                    .as_object()
                    .ok_or("Preset must be a string or object")?
                    .get("type")
                    .ok_or("Preset type not provided")?
                    .as_str()
                    .ok_or("Preset type must be a string")?
            }
        };
        match preset_type {
            "ram" => {
                init_mem_info();
                RingPreset::Ram
            }
            "swap" => {
                init_mem_info();
                RingPreset::Swap
            }
            "cpu" => {
                init_system_info();
                RingPreset::Cpu
            }
            "battery" => {
                init_system_info();
                RingPreset::Battery
            }
            "disk" => {
                let partition = preset
                    .get("partition")
                    .unwrap_or(&Value::String("/".to_string()))
                    .as_str()
                    .ok_or("partition must be string")?
                    .to_string();
                init_system_info();
                register_disk_partition(partition.clone());
                RingPreset::Disk(partition)
            }
            "custom" => {
                RingPreset::Custom(super::common::from_value::<RingCustom>(preset.clone())?)
            }
            _ => {
                return Err(format!("Unknown preset type: {preset_type}"));
            }
        }
    };
    let common = super::common::from_value::<RingCommon>(d)?;
    Ok(Widget::Ring(Box::new(RingConfig { common, preset })))
}
