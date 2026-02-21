pub mod ring;
pub mod text;
pub mod tray;

use cosmic_text::Color;
use knus::{Decode, DecodeScalar};
use ring::RingConfig;
use text::TextConfig;
use tray::TrayConfig;
use util::color::parse_color;

// =================================== OUTLOOK
#[derive(Debug, Clone, Decode)]
pub struct OutlookMargins {
    #[knus(child, default = dt_margin(), unwrap(argument))]
    pub left: i32,
    #[knus(child, default = dt_margin(), unwrap(argument))]
    pub top: i32,
    #[knus(child, default = dt_margin(), unwrap(argument))]
    pub right: i32,
    #[knus(child, default = dt_margin(), unwrap(argument))]
    pub bottom: i32,
}
fn dt_margin() -> i32 {
    5
}
impl Default for OutlookMargins {
    fn default() -> Self {
        Self {
            left: dt_margin(),
            top: dt_margin(),
            right: dt_margin(),
            bottom: dt_margin(),
        }
    }
}
#[derive(Debug, Decode, Clone)]
pub struct OutlookWindowConfig {
    #[knus(child, default)]
    pub margins: OutlookMargins,
    #[knus(child, default = dt_color(), unwrap(argument, decode_with = parse_color))]
    pub color: Color,
    #[knus(child, default = dt_radius(), unwrap(argument))]
    pub border_radius: i32,
    #[knus(child, default = dt_border_width(), unwrap(argument))]
    pub border_width: i32,
}
impl Default for OutlookWindowConfig {
    fn default() -> Self {
        Self {
            margins: Default::default(),
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

#[derive(Debug, Decode, Clone)]
pub struct OutlookBoardConfig {
    #[knus(child, default)]
    pub margins: OutlookMargins,
    #[knus(child, default = dt_color(), unwrap(argument, decode_with = parse_color))]
    pub color: Color,
    #[knus(child, default = dt_radius(), unwrap(argument))]
    pub border_radius: i32,
}

#[derive(Debug, Clone)]
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
        let mut outlook = Self::default();

        for child in node.children() {
            match argv_str(node, ctx)?.as_ref() {
                "window" => {
                    outlook = Self::Window(OutlookWindowConfig::decode_node(child, ctx)?);
                }
                "board" => {
                    outlook = Self::Board(OutlookBoardConfig::decode_node(child, ctx)?);
                }
                _ => {}
            }
        }

        Ok(outlook)
    }
}

// =================================== GRID
#[derive(Debug, Default, Clone, Copy, DecodeScalar)]
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
#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct BoxedWidgetConfig {
    pub index: [isize; 2],
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
            match argv_str(node, ctx)?.as_ref() {
                "index" => index = [argvi_v(child, ctx, 0)?, argvi_v(child, ctx, 1)?],
                _ => {}
            }
        }

        Ok(Self { index, widget })
    }
}

use crate::kdl::util::{argv_str, argv_v, argvi_v};

// =================================== FINAL
#[derive(Debug, Decode, Clone)]
pub struct BoxConfig {
    #[knus(child, default)]
    pub outlook: Outlook,
    #[knus(child, default = dt_gap(), unwrap(argument))]
    pub gap: f64,
    #[knus(child, default, unwrap(argument))]
    pub align: Align,
    #[knus(children(name = "item"), default)]
    pub items: Vec<BoxedWidgetConfig>,
}
fn dt_gap() -> f64 {
    10.
}
