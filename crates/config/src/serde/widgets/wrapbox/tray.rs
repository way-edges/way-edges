use cosmic_text::{Color, FamilyOwned};
use educe::Educe;
use schemars::JsonSchema;
use serde::Deserialize;
use util::color::COLOR_WHITE;

use super::Align;
use crate::serde::shared::{
    color_translate, dt_family_owned, option_color_translate, schema_color, schema_optional_color,
    FamilyOwnedRef,
};

#[derive(Debug, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum HeaderMenuStack {
    #[default]
    HeaderTop,
    MenuTop,
}

#[derive(Debug, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case")]
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

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct HeaderDrawConfig {
    #[serde(default = "dt_header_font_pixel_height")]
    pub font_pixel_height: i32,
    #[serde(default = "dt_header_text_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub text_color: Color,
}
impl Default for HeaderDrawConfig {
    fn default() -> Self {
        Self {
            font_pixel_height: dt_header_font_pixel_height(),
            text_color: dt_header_text_color(),
        }
    }
}
fn dt_header_font_pixel_height() -> i32 {
    20
}
fn dt_header_text_color() -> Color {
    COLOR_WHITE
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct MenuDrawConfig {
    #[serde(default = "dt_menu_margin")]
    pub margin: [i32; 2],
    #[serde(default = "dt_font_pixel_height")]
    pub font_pixel_height: i32,
    #[serde(default = "dt_menu_icon_size")]
    pub icon_size: i32,
    #[serde(default = "dt_menu_marker_size")]
    pub marker_size: i32,
    #[serde(default = "dt_menu_separator_height")]
    pub separator_height: i32,
    #[serde(default = "dt_menu_border_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub border_color: Color,
    #[serde(default = "dt_menu_text_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub text_color: Color,
    #[serde(default)]
    #[serde(deserialize_with = "option_color_translate")]
    #[schemars(schema_with = "schema_optional_color")]
    pub marker_color: Option<Color>,
}
impl Default for MenuDrawConfig {
    fn default() -> Self {
        Self {
            margin: dt_menu_margin(),
            marker_size: dt_menu_marker_size(),
            font_pixel_height: dt_font_pixel_height(),
            separator_height: dt_menu_separator_height(),
            border_color: dt_menu_border_color(),
            text_color: dt_menu_text_color(),
            icon_size: dt_menu_icon_size(),
            marker_color: None,
        }
    }
}
fn dt_menu_margin() -> [i32; 2] {
    [12, 12]
}
fn dt_font_pixel_height() -> i32 {
    22
}
fn dt_menu_icon_size() -> i32 {
    20
}
fn dt_menu_marker_size() -> i32 {
    20
}
fn dt_menu_separator_height() -> i32 {
    5
}
fn dt_menu_border_color() -> Color {
    COLOR_WHITE
}
fn dt_menu_text_color() -> Color {
    COLOR_WHITE
}

#[derive(Educe, Deserialize, JsonSchema, Clone)]
#[educe(Debug)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct TrayConfig {
    #[serde(default = "dt_family_owned")]
    #[serde(with = "FamilyOwnedRef")]
    pub font_family: FamilyOwned,
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
