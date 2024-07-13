use educe::Educe;
use serde::Deserialize;
use serde_jsonrc::Value;

use crate::config::Widget;

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

#[derive(Educe, Deserialize, Default)]
#[educe(Debug)]
#[serde(default)]
pub struct PASpecificConfig {
    #[serde(default)]
    pub redraw_only_on_pa_change: bool,
}

pub fn visit_config(d: Value) -> Result<Widget, String> {
    let key = d.get("type").unwrap().as_str().unwrap().to_string();
    let slide_cfg = from_value::<SlideConfig>(d.clone())?;
    let speaker_cfg = from_value::<PASpecificConfig>(d)?;
    Ok(Widget::PulseAudio(Box::new(PAConfig {
        slide: slide_cfg,
        pa_conf: speaker_cfg,
        is_sink: key.as_str().eq(NAME_SINK),
    })))
}
