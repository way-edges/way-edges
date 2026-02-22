use cosmic_text::{Color, FamilyOwned};
use educe::Educe;
use knus::{Decode, DecodeScalar};
use util::color::{parse_color, COLOR_WHITE};

use crate::kdl::{
    shared::{dt_family_owned, parse_family_owned, NumMargins},
    util::parse_optional_color,
};

use super::Align;

#[derive(Debug, Default, Clone, DecodeScalar)]
pub enum HeaderMenuStack {
    #[default]
    HeaderTop,
    MenuTop,
}

#[derive(Debug, Default, Clone, DecodeScalar)]
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

#[derive(Debug, Decode, Clone)]
pub struct HeaderDrawConfig {
    #[knus(child, default = dt_header_font_pixel_height(), unwrap(argument))]
    pub font_pixel_height: i32,
    #[knus(child, default = dt_header_text_color(), unwrap(argument, decode_with = parse_color))]
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

#[derive(Debug, Clone, Decode)]
pub struct MenuDrawConfig {
    #[knus(child, default = dt_menu_margin())]
    pub margin: NumMargins,
    #[knus(child, default = dt_font_pixel_height(), unwrap(argument))]
    pub font_pixel_height: i32,
    #[knus(child, default = dt_menu_icon_size(), unwrap(argument))]
    pub icon_size: i32,
    #[knus(child, default = dt_menu_marker_size(), unwrap(argument))]
    pub marker_size: i32,
    #[knus(child, default = dt_menu_separator_height(), unwrap(argument))]
    pub separator_height: i32,
    #[knus(child, default = dt_menu_border_color(), unwrap(argument, decode_with = parse_color))]
    pub border_color: Color,
    #[knus(child, default = dt_menu_text_color(), unwrap(argument, decode_with = parse_color))]
    pub text_color: Color,
    #[knus(child, default, unwrap(argument, decode_with = parse_optional_color))]
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
fn dt_menu_margin() -> NumMargins {
    NumMargins {
        left: 12,
        right: 12,
        top: 12,
        bottom: 12,
    }
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

#[derive(Debug, Decode, Clone)]
pub struct TrayConfig {
    #[knus(child, default = dt_family_owned(), unwrap(argument, decode_with = parse_family_owned))]
    pub font_family: FamilyOwned,
    #[knus(child, default, unwrap(argument))]
    pub icon_theme: Option<String>,
    #[knus(child, default = dt_icon_size(), unwrap(argument))]
    pub icon_size: i32,
    #[knus(child, default = dt_tray_gap(), unwrap(argument))]
    pub tray_gap: i32,
    #[knus(child, default, unwrap(argument))]
    pub grid_align: Align,

    #[knus(child, default, unwrap(argument))]
    pub header_menu_stack: HeaderMenuStack,
    #[knus(child, default, unwrap(argument))]
    pub header_menu_align: HeaderMenuAlign,

    #[knus(child, default)]
    pub header_draw_config: HeaderDrawConfig,
    #[knus(child, default)]
    pub menu_draw_config: MenuDrawConfig,
}
impl Default for TrayConfig {
    fn default() -> Self {
        Self {
            font_family: dt_family_owned(),
            icon_theme: None,
            icon_size: dt_icon_size(),
            tray_gap: dt_tray_gap(),
            grid_align: Align::default(),
            header_menu_stack: HeaderMenuStack::default(),
            header_menu_align: HeaderMenuAlign::default(),
            header_draw_config: HeaderDrawConfig::default(),
            menu_draw_config: MenuDrawConfig::default(),
        }
    }
}

fn dt_icon_size() -> i32 {
    20
}
fn dt_tray_gap() -> i32 {
    2
}
