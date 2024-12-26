use educe::Educe;
use gtk::gdk::RGBA;
use serde::{Deserialize, Deserializer};
use serde_jsonrc::de;
use util::shell::shell_cmd;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Preset {
    Speaker(PulseAudioConfig),
    Microphone(PulseAudioConfig),
    Backlight(BacklightConfig),
    Custom(CustomConfig),
}
impl Default for Preset {
    fn default() -> Self {
        Self::Custom(CustomConfig::default())
    }
}

#[derive(Debug, Deserialize)]
pub struct PulseAudioConfig {
    #[serde(default)]
    pub redraw_only_on_pa_change: bool,
    #[serde(default = "default_mute_color")]
    #[serde(deserialize_with = "super::common::color_translate")]
    pub mute_color: RGBA,
    pub device: Option<String>,
}

fn default_mute_color() -> RGBA {
    RGBA::BLACK
}

#[derive(Debug, Deserialize)]
pub struct BacklightConfig {
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub redraw_only_on_change: bool,
}

#[derive(Educe, Deserialize, Default)]
#[educe(Debug)]
pub struct CustomConfig {
    #[serde(default)]
    pub interval: u64,
    #[serde(default)]
    pub cmd: String,
}
