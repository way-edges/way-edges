use cosmic_text::Color;
use educe::Educe;
use serde::Deserialize;
use util::color::COLOR_WHITE;

use super::super::common;
use super::Align;

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
    pub text_color: Color,
}
impl Default for HeaderDrawConfig {
    fn default() -> Self {
        Self {
            font_pixel_height: 16,
            text_color: COLOR_WHITE,
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
    pub border_color: Color,
    #[serde(deserialize_with = "common::color_translate")]
    pub text_color: Color,
    #[serde(deserialize_with = "common::option_color_translate")]
    pub marker_color: Option<Color>,
}
impl Default for MenuDrawConfig {
    fn default() -> Self {
        Self {
            margin: [12, 12],
            marker_size: 20,
            font_pixel_height: 24,
            separator_height: 5,
            border_color: COLOR_WHITE,
            text_color: COLOR_WHITE,
            marker_color: None,
        }
    }
}

#[derive(Educe, Deserialize)]
#[educe(Debug)]
pub struct TrayConfig {
    #[serde(default)]
    pub icon_theme: Option<String>,
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
