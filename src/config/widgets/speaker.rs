use educe::Educe;
use serde::Deserialize;
use serde_jsonrc::Value;

use crate::config::Widget;

use super::{common::from_value, slide::SlideConfig};

#[derive(Educe)]
#[educe(Debug)]
pub struct SpeakerConfig {
    pub slide: SlideConfig,
    pub speaker: SpeakerSpecificConfig,
}

#[derive(Educe, Deserialize, Default)]
#[educe(Debug)]
#[serde(default)]
pub struct SpeakerSpecificConfig {
    #[serde(default)]
    pub redraw_only_on_pa_change: bool,
}

pub fn visit_config(d: Value) -> Result<Widget, String> {
    let slide_cfg = from_value::<SlideConfig>(d.clone())?;
    let speaker_cfg = from_value::<SpeakerSpecificConfig>(d)?;
    Ok(Widget::Speaker(Box::new(SpeakerConfig {
        slide: slide_cfg,
        speaker: speaker_cfg,
    })))
}
