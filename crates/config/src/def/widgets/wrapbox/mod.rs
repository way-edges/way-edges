pub mod ring;
pub mod text;
pub mod tray;

use cosmic_text::Color;
use knus::{Decode, DecodeScalar};
use ring::RingConfig;
use text::TextConfig;
use tray::TrayConfig;
use util::color::parse_color;

// Add serde imports
use crate::def::shared::{color_translate, schema_color};
use schemars::{JsonSchema, Schema};
use serde::Deserialize;
use serde_json::Value;
use way_edges_derive::const_property;

// =================================== OUTLOOK
fn dt_outlook_margin() -> NumMargins {
    NumMargins {
        left: 5,
        right: 5,
        top: 5,
        bottom: 5,
    }
}
#[derive(Debug, Decode, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct OutlookWindowConfig {
    #[knus(child, default = dt_outlook_margin())]
    #[serde(default = "dt_outlook_margin")]
    pub margins: NumMargins,
    #[knus(child, default = dt_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub color: Color,
    #[knus(child, default = dt_radius(), unwrap(argument))]
    #[serde(default = "dt_radius")]
    pub border_radius: i32,
    #[knus(child, default = dt_border_width(), unwrap(argument))]
    #[serde(default = "dt_border_width")]
    pub border_width: i32,
}
impl Default for OutlookWindowConfig {
    fn default() -> Self {
        Self {
            margins: dt_outlook_margin(),
            color: dt_color(),
            border_radius: dt_radius(),
            border_width: dt_border_width(),
        }
    }
}
fn dt_color() -> Color {
    parse_color("#4d8080").unwrap()
}
fn dt_radius() -> i32 {
    5
}
fn dt_border_width() -> i32 {
    15
}

#[derive(Debug, Decode, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct OutlookBoardConfig {
    #[knus(child, default = dt_outlook_margin())]
    #[serde(default = "dt_outlook_margin")]
    pub margins: NumMargins,
    #[knus(child, default = dt_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub color: Color,
    #[knus(child, default = dt_radius(), unwrap(argument))]
    #[serde(default = "dt_radius")]
    pub border_radius: i32,
}

#[derive(Debug, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum Outlook {
    Window(OutlookWindowConfig),
    Board(OutlookBoardConfig),
}
impl Default for Outlook {
    fn default() -> Self {
        Self::Window(OutlookWindowConfig::default())
    }
}
impl<S: knus::traits::ErrorSpan> knus::Decode<S> for Outlook {
    fn decode_node(
        node: &knus::ast::SpannedNode<S>,
        ctx: &mut knus::decode::Context<S>,
    ) -> Result<Self, knus::errors::DecodeError<S>> {
        match argv_str(node, ctx)?.as_ref() {
            "window" => Ok(Self::Window(OutlookWindowConfig::decode_node(node, ctx)?)),
            "board" => Ok(Self::Board(OutlookBoardConfig::decode_node(node, ctx)?)),
            name => Err(knus::errors::DecodeError::unexpected(
                &node.node_name,
                "window or board",
                format!("Unknown outlook type: {name}"),
            )),
        }
    }
}

// =================================== GRID
#[derive(Debug, Default, Clone, Copy, DecodeScalar, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Align {
    #[default]
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    CenterCenter,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

pub type AlignFuncPos = (f64, f64);
pub type AlignFuncGridBlockSize = (f64, f64);
pub type AlignFuncContentSize = (f64, f64);
pub type AlignFunc =
    Box<fn(AlignFuncPos, AlignFuncGridBlockSize, AlignFuncContentSize) -> AlignFuncPos>;

impl Align {
    pub fn to_func(&self) -> AlignFunc {
        macro_rules! align_y {
            (T, $pos:expr, $size:expr, $content_size:expr) => {
                $pos.1
            };
            (C, $pos:expr, $size:expr, $content_size:expr) => {
                $pos.1 + ($size.1 - $content_size.1) / 2.
            };
            (B, $pos:expr, $size:expr, $content_size:expr) => {
                $pos.1 + ($size.1 - $content_size.1)
            };
        }

        macro_rules! align_x {
            (L, $pos:expr, $size:expr, $content_size:expr) => {
                $pos.0
            };
            (C, $pos:expr, $size:expr, $content_size:expr) => {
                $pos.0 + ($size.0 - $content_size.0) / 2.
            };
            (R, $pos:expr, $size:expr, $content_size:expr) => {
                $pos.0 + ($size.0 - $content_size.0)
            };
        }

        macro_rules! a {
            ($x:tt $y:tt) => {
                |pos, size, content_size| {
                    (
                        align_x!($x, pos, size, content_size),
                        align_y!($y, pos, size, content_size),
                    )
                }
            };
        }

        Box::new(match self {
            #[allow(unused)]
            Align::TopLeft => a!(L T),
            Align::TopCenter => a!(C T),
            Align::TopRight => a!(R T),
            Align::CenterLeft => a!(L C),
            Align::CenterCenter => a!(C C),
            Align::CenterRight => a!(R C),
            Align::BottomLeft => a!(L B),
            Align::BottomCenter => a!(C B),
            Align::BottomRight => a!(R B),
        })
    }
}

// =================================== WIDGETS
#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum BoxedWidget {
    Ring(RingConfig),
    Text(TextConfig),
    Tray(TrayConfig),
}

impl<S: knus::traits::ErrorSpan> knus::Decode<S> for BoxedWidget {
    fn decode_node(
        node: &knus::ast::SpannedNode<S>,
        ctx: &mut knus::decode::Context<S>,
    ) -> Result<Self, knus::errors::DecodeError<S>> {
        let widget = match argv_str(node, ctx)?.as_ref() {
            "ring" => Self::Ring(RingConfig::decode_node(node, ctx)?),
            "text" => Self::Text(TextConfig::decode_node(node, ctx)?),
            "tray" => Self::Tray(TrayConfig::decode_node(node, ctx)?),
            name => {
                return Err(knus::errors::DecodeError::unexpected(
                    &node.node_name,
                    "ring, text or tray",
                    format!("Unknown widget type: {name}"),
                ))
            }
        };
        Ok(widget)
    }
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct BoxedWidgetConfig {
    #[serde(default = "dt_index")]
    pub index: [isize; 2],
    #[serde(flatten)]
    pub widget: BoxedWidget,
}
fn dt_index() -> [isize; 2] {
    [-1, -1]
}

impl<S: knus::traits::ErrorSpan> knus::Decode<S> for BoxedWidgetConfig {
    fn decode_node(
        node: &knus::ast::SpannedNode<S>,
        ctx: &mut knus::decode::Context<S>,
    ) -> Result<Self, knus::errors::DecodeError<S>> {
        let widget = BoxedWidget::decode_node(node, ctx)?;
        let mut index = dt_index();

        for child in node.children() {
            if child.node_name.as_ref() == "index" {
                index = [argvi_v(child, ctx, 0)?, argvi_v(child, ctx, 1)?]
            }
        }

        Ok(Self { index, widget })
    }
}

use crate::def::{
    shared::NumMargins,
    util::{argv_str, argvi_v},
};

// =================================== FINAL
#[derive(Debug, Decode, Deserialize, JsonSchema, Clone)]
#[schemars(deny_unknown_fields)]
#[schemars(transform = BoxConfig_generate_defs)]
#[const_property("type", "wrap-box")]
#[serde(rename_all = "kebab-case")]
pub struct BoxConfig {
    #[knus(child, default)]
    #[serde(default)]
    pub outlook: Outlook,
    #[knus(child, default = dt_gap(), unwrap(argument))]
    #[serde(default = "dt_gap")]
    pub gap: f64,
    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub align: Align,
    #[knus(children(name = "item"), default)]
    #[serde(default)]
    pub items: Vec<BoxedWidgetConfig>,
}
fn dt_gap() -> f64 {
    10.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_minimal_box_config() {
        let kdl = r#"
wrap-box {
    edge "bottom"
    thickness 20
    length "40%"
}
"#;
        let parsed: Vec<crate::def::WidgetConf> = knus::parse("test", kdl).unwrap();
        if let crate::def::WidgetConf::WrapBox(wrap_box) = &parsed[0] {
            // Assert defaults
            assert_eq!(wrap_box.widget.gap, dt_gap());
            assert!(matches!(wrap_box.widget.align, Align::TopLeft));
            assert!(wrap_box.widget.items.is_empty());
            // Outlook defaults
            match &wrap_box.widget.outlook {
                Outlook::Window(config) => {
                    assert_eq!(config.margins, dt_outlook_margin());
                    assert_eq!(config.color, dt_color());
                    assert_eq!(config.border_radius, dt_radius());
                    assert_eq!(config.border_width, dt_border_width());
                }
                _ => panic!("Expected Window outlook"),
            }
        } else {
            panic!("Expected WrapBox");
        }
    }

    #[test]
    fn test_decode_box_config_with_outlook_window() {
        let kdl = r##"
wrap-box {
    edge "bottom"
    thickness 20
    length "40%"
    outlook "window" {
    }
}
"##;
        let parsed: Vec<crate::def::WidgetConf> = knus::parse("test", kdl).unwrap();
        if let crate::def::WidgetConf::WrapBox(wrap_box) = &parsed[0] {
            match &wrap_box.widget.outlook {
                Outlook::Window(_) => {
                    // Just check that it's Window
                }
                _ => panic!("Expected Window outlook"),
            }
        } else {
            panic!("Expected WrapBox");
        }
    }

    #[test]
    fn test_decode_full_box_config() {
        let kdl = r##"
wrap-box {
    edge "bottom"
    thickness 20
    length "40%"
    outlook "window" {
        margins {
            left 10
            right 10
            top 10
            bottom 10
        }
        color "#ffffff"
        border-radius 10
        border-width 20
    }
    gap 15.0
    align "center-center"
    item "ring" {
        index 0 1

        preset "custom" {
        }
    }
    item "text" {
        index 1 0

        preset "custom" {
        }
    }
}
"##;
        let parsed: Vec<crate::def::WidgetConf> = knus::parse("test", kdl).unwrap();
        if let crate::def::WidgetConf::WrapBox(wrap_box) = &parsed[0] {
            let config = &wrap_box.widget;
            // Check outlook
            match &config.outlook {
                Outlook::Window(window_config) => {
                    assert_eq!(
                        window_config.margins,
                        NumMargins {
                            left: 10,
                            right: 10,
                            top: 10,
                            bottom: 10
                        }
                    );
                    assert_eq!(window_config.color, parse_color("#ffffff").unwrap());
                    assert_eq!(window_config.border_radius, 10);
                    assert_eq!(window_config.border_width, 20);
                }
                _ => panic!("Expected Window outlook"),
            }
            // Check gap
            assert_eq!(config.gap, 15.0);
            // Check align
            assert!(matches!(config.align, Align::CenterCenter));
            // Check items
            assert_eq!(config.items.len(), 2);
            assert_eq!(config.items[0].index, [0, 1]);
            assert!(matches!(config.items[0].widget, BoxedWidget::Ring(_)));
            assert_eq!(config.items[1].index, [1, 0]);
            assert!(matches!(config.items[1].widget, BoxedWidget::Text(_)));
        } else {
            panic!("Expected WrapBox");
        }
    }
}
