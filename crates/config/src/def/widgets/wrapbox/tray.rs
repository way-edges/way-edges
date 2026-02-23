use cosmic_text::{Color, FamilyOwned};
use knus::{Decode, DecodeScalar};
use schemars::JsonSchema;
use serde::Deserialize;
use util::color::{parse_color, COLOR_WHITE};

use crate::def::{
    shared::{
        color_translate, deserialize_family_owned, dt_family_owned, option_color_translate,
        parse_family_owned, schema_color, schema_family_owned, schema_optional_color, NumMargins,
    },
    util::parse_optional_color,
};

use super::Align;

#[derive(Debug, Default, Clone, DecodeScalar, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum HeaderMenuStack {
    #[default]
    HeaderTop,
    MenuTop,
}

#[derive(Debug, Default, Clone, DecodeScalar, Deserialize, JsonSchema)]
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

#[derive(Debug, Decode, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[schemars(deny_unknown_fields)]
pub struct HeaderDrawConfig {
    #[knus(child, default = dt_header_font_pixel_height(), unwrap(argument))]
    #[serde(default = "dt_header_font_pixel_height")]
    pub font_pixel_height: i32,
    #[knus(child, default = dt_header_text_color(), unwrap(argument, decode_with = parse_color))]
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

#[derive(Debug, Clone, Decode, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[schemars(deny_unknown_fields)]
pub struct MenuDrawConfig {
    #[knus(child, default = dt_menu_margin())]
    #[serde(default = "dt_menu_margin")]
    pub margin: NumMargins,
    #[knus(child, default = dt_font_pixel_height(), unwrap(argument))]
    #[serde(default = "dt_font_pixel_height")]
    pub font_pixel_height: i32,
    #[knus(child, default = dt_menu_icon_size(), unwrap(argument))]
    #[serde(default = "dt_menu_icon_size")]
    pub icon_size: i32,
    #[knus(child, default = dt_menu_marker_size(), unwrap(argument))]
    #[serde(default = "dt_menu_marker_size")]
    pub marker_size: i32,
    #[knus(child, default = dt_menu_separator_height(), unwrap(argument))]
    #[serde(default = "dt_menu_separator_height")]
    pub separator_height: i32,
    #[knus(child, default = dt_menu_border_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_menu_border_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub border_color: Color,
    #[knus(child, default = dt_menu_text_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_menu_text_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub text_color: Color,
    #[knus(child, default, unwrap(argument, decode_with = parse_optional_color))]
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

#[derive(Debug, Decode, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[schemars(deny_unknown_fields)]
pub struct TrayConfig {
    #[knus(child, default = dt_family_owned(), unwrap(argument, decode_with = parse_family_owned))]
    #[serde(default = "dt_family_owned")]
    #[serde(deserialize_with = "deserialize_family_owned")]
    #[schemars(schema_with = "schema_family_owned")]
    pub font_family: FamilyOwned,
    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub icon_theme: Option<String>,
    #[knus(child, default = dt_icon_size(), unwrap(argument))]
    #[serde(default = "dt_icon_size")]
    pub icon_size: i32,
    #[knus(child, default = dt_tray_gap(), unwrap(argument))]
    #[serde(default = "dt_tray_gap")]
    pub tray_gap: i32,
    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub grid_align: Align,

    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub header_menu_stack: HeaderMenuStack,
    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub header_menu_align: HeaderMenuAlign,

    #[knus(child, default)]
    #[serde(default)]
    pub header_draw_config: HeaderDrawConfig,
    #[knus(child, default)]
    #[serde(default)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_tray_configs() {
        let kdl = r##"
wrap-box {
    edge "bottom"
    thickness 20
    length "40%"
    item "tray" {
        index 0 0
        icon-theme "Papirus"
        icon-size 24
        tray-gap 5
        grid-align "center-center"
        header-menu-stack "menu-top"
        header-menu-align "right"
        header-draw-config {
            font-pixel-height 25
            text-color "#ff0000"
        }
        menu-draw-config {
            margin {
                left 10
                right 10
                top 10
                bottom 10
            }
            font-pixel-height 26
            icon-size 22
            marker-size 18
            separator-height 4
            border-color "#00ff00"
            text-color "#0000ff"
            marker-color "#ffff00"
        }
    }
}
"##;
        let parsed: Vec<crate::def::WidgetConf> = knus::parse("test", kdl).unwrap();
        if let crate::def::WidgetConf::WrapBox(wrap_box) = &parsed[0] {
            let config = &wrap_box.widget;
            assert_eq!(config.items.len(), 1);

            // Tray with custom fields
            if let crate::def::widgets::wrapbox::BoxedWidget::Tray(tray_config) =
                &config.items[0].widget
            {
                assert_eq!(config.items[0].index, [0, 0]);
                assert_eq!(tray_config.icon_theme.as_ref().unwrap(), "Papirus");
                assert_eq!(tray_config.icon_size, 24);
                assert_eq!(tray_config.tray_gap, 5);
                assert!(matches!(tray_config.grid_align, Align::CenterCenter));
                assert!(matches!(
                    tray_config.header_menu_stack,
                    HeaderMenuStack::MenuTop
                ));
                assert!(matches!(
                    tray_config.header_menu_align,
                    HeaderMenuAlign::Right
                ));
                assert_eq!(tray_config.header_draw_config.font_pixel_height, 25);
                assert_eq!(
                    tray_config.header_draw_config.text_color,
                    parse_color("#ff0000").unwrap()
                );
                assert_eq!(tray_config.menu_draw_config.margin.left, 10);
                assert_eq!(tray_config.menu_draw_config.margin.right, 10);
                assert_eq!(tray_config.menu_draw_config.margin.top, 10);
                assert_eq!(tray_config.menu_draw_config.margin.bottom, 10);
                assert_eq!(tray_config.menu_draw_config.font_pixel_height, 26);
                assert_eq!(tray_config.menu_draw_config.icon_size, 22);
                assert_eq!(tray_config.menu_draw_config.marker_size, 18);
                assert_eq!(tray_config.menu_draw_config.separator_height, 4);
                assert_eq!(
                    tray_config.menu_draw_config.border_color,
                    parse_color("#00ff00").unwrap()
                );
                assert_eq!(
                    tray_config.menu_draw_config.text_color,
                    parse_color("#0000ff").unwrap()
                );
                assert_eq!(
                    tray_config.menu_draw_config.marker_color.unwrap(),
                    parse_color("#ffff00").unwrap()
                );
            } else {
                panic!("Expected Tray widget");
            }
        } else {
            panic!("Expected WrapBox");
        }
    }
}
