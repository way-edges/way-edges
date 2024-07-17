use educe::Educe;
use serde::Deserialize;
use serde_jsonrc::Value;

use crate::config::Widget;

use super::{common::from_value, slide::SlideConfig};

pub const NAME: &str = "backlight";

#[derive(Educe)]
#[educe(Debug)]
pub struct BLConfig {
    pub slide: SlideConfig,
    pub bl_conf: BLSpecificConfig,
}

#[derive(Educe, Deserialize)]
#[educe(Debug)]
pub struct BLSpecificConfig {
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub redraw_only_on_change: bool,
}

pub fn visit_config(d: Value) -> Result<Widget, String> {
    let slide_cfg = from_value::<SlideConfig>(d.clone())?;
    let bl_conf = from_value::<BLSpecificConfig>(d)?;
    Ok(Widget::Backlight(Box::new(BLConfig {
        slide: slide_cfg,
        bl_conf,
    })))
}
