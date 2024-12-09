use educe::Educe;
use gtk::gdk::RGBA;
use serde::Deserialize;
use serde_jsonrc::Value;

use crate::config::widgets::common;
use crate::config::widgets::common::from_value;

use super::{Align, BoxedWidget};

pub const NAME: &str = "tray";

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(rename_all = "snake_case")]
pub enum HeaderMenuStack {
    #[default]
    HeaderTop,
    MenuTop,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(rename_all = "snake_case")]
pub enum HeaderMenuAlign {
    #[default]
    Left,
    Right,
}
impl HeaderMenuAlign {
    pub fn is_left(&self) -> bool {
        match self {
            HeaderMenuAlign::Left => true,
            HeaderMenuAlign::Right => false,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct HeaderDrawConfig {
    pub font_pixel_height: i32,
    #[serde(deserialize_with = "common::color_translate")]
    pub text_color: RGBA,
}
impl Default for HeaderDrawConfig {
    fn default() -> Self {
        Self {
            font_pixel_height: 16,
            text_color: RGBA::WHITE,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct MenuDrawConfig {
    pub margin: [i32; 2],
    pub font_pixel_height: i32,
    pub marker_size: i32,
    pub separator_height: i32,
    #[serde(deserialize_with = "common::color_translate")]
    pub border_color: RGBA,
    #[serde(deserialize_with = "common::color_translate")]
    pub text_color: RGBA,
    #[serde(deserialize_with = "common::option_color_translate")]
    pub marker_color: Option<RGBA>,
}
impl Default for MenuDrawConfig {
    fn default() -> Self {
        Self {
            margin: [12, 16],
            marker_size: 20,
            font_pixel_height: 20,
            separator_height: 5,
            border_color: RGBA::WHITE,
            text_color: RGBA::WHITE,
            marker_color: None,
        }
    }
}

#[derive(Educe, Deserialize)]
#[educe(Debug)]
pub struct TrayConfig {
    #[serde(default = "dt_icon_size")]
    pub icon_size: i32,
    #[serde(default = "dt_tray_gap")]
    pub tray_gap: i32,
    #[serde(default)]
    pub grid_align: Align,

    #[serde(default)]
    pub header_menu_stack: HeaderMenuStack,
    #[serde(default)]
    pub header_menu_align: HeaderMenuAlign,

    #[serde(default)]
    pub header_draw_config: HeaderDrawConfig,
    #[serde(default)]
    pub menu_draw_config: MenuDrawConfig,
}

fn dt_icon_size() -> i32 {
    20
}
fn dt_tray_gap() -> i32 {
    2
}

pub fn visit_config(v: Value) -> Result<BoxedWidget, String> {
    let conf: TrayConfig = from_value(v)?;
    Ok(BoxedWidget::Tray(Box::new(conf)))
}
