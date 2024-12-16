use educe::Educe;
use gtk::gdk::RGBA;
use serde::Deserialize;
use serde_jsonrc::Value;

use crate::{widgets::common, Widget};

use super::{common::from_value, slide::SlideConfig};

pub const NAME_SINK: &str = "speaker";
pub const NAME_SOUCE: &str = "microphone";

#[derive(Educe)]
#[educe(Debug)]
pub struct PAConfig {
    pub slide: SlideConfig,
    pub pa_conf: PASpecificConfig,
    pub is_sink: bool,
}

#[derive(Educe, Deserialize)]
#[educe(Debug)]
pub struct PASpecificConfig {
    #[serde(default)]
    pub redraw_only_on_pa_change: bool,
    #[serde(default = "default_mute_color")]
    #[serde(deserialize_with = "common::color_translate")]
    pub mute_color: RGBA,
    pub device: Option<String>,
}

fn default_mute_color() -> RGBA {
    RGBA::BLACK
}

pub fn visit_config(d: Value) -> Result<Widget, String> {
    let key = d.get("type").unwrap().as_str().unwrap().to_string();
    let slide_cfg = from_value::<SlideConfig>(d.clone())?;
    let pa_conf = from_value::<PASpecificConfig>(d)?;
    Ok(Widget::PulseAudio(Box::new(PAConfig {
        slide: slide_cfg,
        pa_conf,
        is_sink: key.as_str().eq(NAME_SINK),
    })))
}
