pub mod ring;
pub mod text;
pub mod tray;

use cosmic_text::Color;
use ring::RingConfig;
use serde::Deserialize;
use text::TextConfig;
use tray::TrayConfig;
use util::color::parse_color;

// =================================== OUTLOOK
#[derive(Debug, Deserialize, Clone)]
pub struct OutlookMargins {
    #[serde(default = "dt_margin")]
    pub left: i32,
    #[serde(default = "dt_margin")]
    pub top: i32,
    #[serde(default = "dt_margin")]
    pub right: i32,
    #[serde(default = "dt_margin")]
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
#[derive(Debug, Deserialize)]
pub struct OutlookWindowConfig {
    #[serde(default)]
    pub margins: OutlookMargins,
    #[serde(default = "dt_color")]
    #[serde(deserialize_with = "super::common::color_translate")]
    pub color: Color,
    #[serde(default = "dt_radius")]
    pub border_radius: i32,
    #[serde(default = "dt_border_width")]
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Outlook {
    Window(OutlookWindowConfig),
}
impl Default for Outlook {
    fn default() -> Self {
        Self::Window(OutlookWindowConfig::default())
    }
}

// =================================== GRID
#[derive(Deserialize, Debug, Default, Clone, Copy)]
#[serde(rename_all = "snake_case")]
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
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum BoxedWidget {
    Ring(RingConfig),
    Text(TextConfig),
    Tray(TrayConfig),
}

#[derive(Debug, Deserialize)]
pub struct BoxedWidgetConfig {
    pub index: [isize; 2],
    pub widget: BoxedWidget,
}

// =================================== FINAL
#[derive(Debug, Deserialize)]
pub struct BoxConfig {
    #[serde(default)]
    pub outlook: Outlook,
    #[serde(default)]
    pub widgets: Vec<BoxedWidgetConfig>,

    #[serde(default = "dt_gap")]
    pub gap: f64,
    #[serde(default)]
    pub align: Align,
}
fn dt_gap() -> f64 {
    10.
}
