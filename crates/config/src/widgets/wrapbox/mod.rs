pub mod ring;
pub mod text;
pub mod tray;

use std::str::FromStr;

use gtk::gdk::RGBA;
use ring::RingConfig;
use serde::Deserialize;
use text::TextConfig;
use tray::TrayConfig;

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
    pub color: RGBA,
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
fn dt_color() -> RGBA {
    RGBA::from_str("#4d8080").unwrap()
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

pub mod common {
    use std::{borrow::Cow, str::FromStr};

    use serde::Deserialize;

    use util::notify_send;

    #[derive(Debug)]
    pub struct FloatNumTemplate {
        precision: usize,
        multiply: Option<f64>,
    }
    impl Default for FloatNumTemplate {
        fn default() -> Self {
            Self {
                precision: 2,
                multiply: None,
            }
        }
    }
    impl FloatNumTemplate {
        pub fn parse(&self, mut progress: f64) -> String {
            if let Some(multiply) = self.multiply {
                progress *= multiply
            }
            format!("{:.precision$}", progress, precision = self.precision)
        }
    }
    impl FromStr for FloatNumTemplate {
        type Err = String;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let mut template = Self::default();

            let s = s.trim();
            if s.is_empty() {
                return Ok(template);
            }

            let mut precision = s;
            let mut multiply = None;

            if let Some((p, m)) = s.split_once(',') {
                let p = p.trim();
                let m = m.trim();

                precision = p;
                if !m.is_empty() {
                    multiply = Some(m);
                }
            }

            if !precision.is_empty() {
                template.precision = precision.parse::<usize>().map_err(|e| e.to_string())?;
            }

            if let Some(multiply) = multiply {
                template.multiply = Some(multiply.parse::<f64>().map_err(|e| e.to_string())?);
            }

            Ok(template)
        }
    }

    #[derive(Debug)]
    pub enum AvailableRingTemplate {
        Preset,
        Float(FloatNumTemplate),
    }

    #[derive(Debug)]
    pub enum TemplateContent {
        String(String),
        Template(AvailableRingTemplate),
    }

    #[derive(Debug)]
    pub struct Template {
        pub contents: Vec<TemplateContent>,
    }
    impl<'de> Deserialize<'de> for Template {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            struct TemplateVisiter;
            impl serde::de::Visitor<'_> for TemplateVisiter {
                type Value = Template;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("Failed to parse template")
                }
                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Template::from_str(v).map_err(serde::de::Error::custom)
                }

                fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    self.visit_str(v.as_str())
                }
            }

            deserializer.deserialize_str(TemplateVisiter)
        }
    }

    impl FromStr for Template {
        type Err = String;

        fn from_str(raw: &str) -> Result<Self, Self::Err> {
            let mut contents = vec![];
            let mut record_index = 0;

            let matches = extract_braces(raw)?;
            for m in matches {
                // push string before
                let end = m.start;
                if end > record_index {
                    contents.push(TemplateContent::String(
                        raw[record_index..end].replace(r"\", ""),
                    ))
                }
                record_index = m.end;

                // push template
                let template = m.get_content();
                let (name, arg) = match template.split_once(":") {
                    Some((n, a)) => (n.trim(), a.trim()),
                    None => (template.trim(), ""),
                };
                let template = match name {
                    "float" => {
                        let Ok(template) = arg.parse::<FloatNumTemplate>() else {
                            let msg = format!("failed to parse process template: {arg}");
                            notify_send("Way-Eges", &msg, true);
                            log::error!("{msg}");
                            continue;
                        };
                        AvailableRingTemplate::Float(template)
                    }
                    "preset" => AvailableRingTemplate::Preset,
                    _ => {
                        let msg = format!("Unknown template: {name}");
                        notify_send("Way-Eges", &msg, true);
                        log::error!("{msg}");
                        continue;
                    }
                };

                contents.push(TemplateContent::Template(template));
            }

            if record_index < raw.len() {
                contents.push(TemplateContent::String(
                    raw[record_index..].replace(r"\", ""),
                ));
            };

            Ok(Self { contents })
        }
    }

    impl Template {
        pub fn parse(&self, arg: TemplateArg) -> String {
            self.contents
                .iter()
                .filter_map(|content| match content {
                    TemplateContent::String(s) => Some(Cow::Borrowed(s.as_str())),
                    TemplateContent::Template(available_ring_template) => {
                        match available_ring_template {
                            AvailableRingTemplate::Preset => arg.preset.map(Cow::Borrowed),
                            AvailableRingTemplate::Float(temp) => {
                                arg.float.map(|progress| Cow::Owned(temp.parse(progress)))
                            }
                        }
                    }
                })
                .collect::<Vec<Cow<str>>>()
                .join("")
                .to_string()
        }
    }

    #[derive(Debug, Clone)]
    pub struct TemplateArg<'a> {
        pub float: Option<f64>,
        pub preset: Option<&'a str>,
    }

    struct BraceMatch<'a> {
        start: usize,
        end: usize,
        s: &'a str,
    }
    impl<'a> BraceMatch<'a> {
        fn from_total(start: usize, end: usize, total: &'a str) -> Self {
            let s = &total[start..end];
            Self { start, end, s }
        }
        fn get_content(&self) -> &str {
            &self.s[1..self.s.len() - 1]
        }
    }

    fn extract_braces(input: &str) -> Result<Vec<BraceMatch<'_>>, String> {
        let chars = input.chars().enumerate();

        struct BraceState<'a> {
            start: i32,
            indexes: Vec<BraceMatch<'a>>,
            str: &'a str,
        }
        impl<'a> BraceState<'a> {
            fn new(s: &'a str) -> Self {
                Self {
                    start: -1,
                    indexes: vec![],
                    str: s,
                }
            }
            fn enter(&mut self, index: i32) {
                self.start = index;
            }
            fn leave(&mut self, index: i32) {
                if self.start != -1 {
                    self.indexes.push(BraceMatch::from_total(
                        self.start as usize,
                        (index + 1) as usize,
                        self.str,
                    ));
                }
                self.start = -1
            }
        }

        let mut state = BraceState::new(input);
        let mut escaped = false;

        for (index, c) in chars {
            match c {
                '\\' => {
                    escaped = !escaped;
                }
                '{' if !escaped => {
                    state.enter(index as i32);
                }
                '}' if !escaped => {
                    state.leave(index as i32);
                }
                _ => {}
            }
        }
        let BraceState {
            start: _,
            indexes,
            str: _,
        } = state;

        Ok(indexes)
    }

    mod test {
        #[allow(unused_imports)]
        use super::*;

        #[test]
        fn test_float_template() {
            macro_rules! test {
                ($i:expr) => {
                    let res = FloatNumTemplate::from_str($i).unwrap().parse(0.5125);
                    assert_eq!(res, "0.51");
                };
            }
            test!(",");
            test!("");
            test!("2,");
            test!("2");
            test!(" 2,");
            test!(" 2");
            test!("2,1");
            test!("2, 1");
            test!(" 2, 1 ");
            test!(" , 1 ");
        }

        #[test]
        fn test_float_template_parse() {
            macro_rules! test {
                ($i:expr, $s:expr) => {
                    let res = FloatNumTemplate::from_str($i).unwrap().parse(0.5125);
                    assert_eq!(res, $s);
                };
            }
            test!("0", "1");
            test!("1", "0.5");
            test!("2", "0.51");
            test!("4", "0.5125");
            test!("10", "0.5125000000");
            test!(",0", "0.00");
            test!("3,2", "1.025");
            test!(",10", "5.12");
            test!(",100", "51.25");
        }

        #[test]
        fn test_ring_template() {
            macro_rules! test {
                ($i:expr, $s:expr) => {
                    let len = Template::from_str($i).unwrap().contents.len();
                    assert_eq!(len, $s);
                };
            }
            // test!("{}", 0);
            test!(r"\{\}", 1);
            test!(r"\{}", 1);
            test!(r"{\}", 1);
            test!("{preset:}{float:}", 2);
            test!("{preset}{float}", 2);
            test!(" {preset}{float}", 3);
            test!("{preset} {float}", 3);
            test!("{preset}{float} ", 3);
            test!(" {preset} {float} ", 5);
            test!("  { preset }  { float }  ", 5);
            test!("{{preset}}{float}", 4);
            test!(r"\{preset\} \{float\}", 1);
            test!(r"\{preset\} {float\}", 1);
            test!("{{preset}}{{float}", 4);
        }

        #[test]
        fn test_parse_content() {
            macro_rules! test {
                ($i:expr, $a:expr, $s:expr) => {
                    let res = Template::from_str($i).unwrap().parse($a);
                    assert_eq!(res, $s);
                };
            }
            test!(
                "",
                TemplateArg {
                    float: Some(2.),
                    preset: Some("hh")
                },
                ""
            );
            test!(
                "a",
                TemplateArg {
                    float: Some(2.),
                    preset: Some("hh")
                },
                "a"
            );
            // test!(
            //     "{}{}",
            //     TemplateArg {
            //         float: Some(2.),
            //         preset: Some("hh")
            //     },
            //     ""
            // );
            test!(
                "{float}",
                TemplateArg {
                    float: Some(2.),
                    preset: Some("hh")
                },
                "2.00"
            );
            test!(
                " { preset}a",
                TemplateArg {
                    float: Some(2.),
                    preset: Some("hh")
                },
                " hha"
            );
        }
    }
}
