pub mod base;
pub mod preset;

use base::SlideConfig;
use serde_jsonrc::Value;

use super::common::{self, from_value};
use super::Widget;

pub const NAME: &str = "slide";

pub fn visit_config(d: Value) -> Result<Widget, String> {
    let c = from_value::<SlideConfig>(d)?;
    Ok(Widget::Slider(Box::new(c)))
}
