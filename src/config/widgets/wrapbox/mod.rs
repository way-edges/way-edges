pub mod ring;
pub mod text;

use std::str::FromStr;

use educe::Educe;
use gtk::gdk::RGBA;
use ring::RingConfig;
use serde::{Deserialize, Deserializer};
use serde_jsonrc::{Map, Value};
use text::TextConfig;

use crate::config::{NumOrRelative, Widget};

use super::common::from_value;

pub const NAME: &str = "box";

#[derive(Debug, Deserialize)]
pub struct OutlookWindowConfig {
    #[serde(default = "dt_margins")]
    pub margins: Option<[i32; 4]>,
    #[serde(default = "dt_color")]
    #[serde(deserialize_with = "super::common::color_translate")]
    pub color: RGBA,
    #[serde(default = "dt_radius")]
    pub border_radius: f64,
    #[serde(default = "dt_border_width")]
    pub border_width: i32,
}
fn dt_margins() -> Option<[i32; 4]> {
    Some([5, 5, 5, 5])
}
fn dt_color() -> RGBA {
    RGBA::from_str("#4d8080").unwrap()
}
fn dt_radius() -> f64 {
    5.
}
fn dt_border_width() -> i32 {
    15
}

#[derive(Debug, Deserialize)]
pub enum Outlook {
    Window(OutlookWindowConfig),
}

#[derive(Debug)]
pub struct BoxedWidgetConfig {
    pub index: [isize; 2],
    pub widget: BoxedWidget,
}

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

#[derive(Deserialize, Debug)]
pub struct BoxSelf {
    #[serde(default = "dt_gap")]
    pub gap: f64,
    #[serde(default)]
    pub align: Align,
    #[serde(default = "super::common::dt_extra_trigger_size")]
    pub extra_trigger_size: NumOrRelative,
    #[serde(default = "super::common::dt_transition_duration")]
    pub transition_duration: u64,
    #[serde(default = "super::common::dt_frame_rate")]
    pub frame_rate: u32,
}

fn dt_gap() -> f64 {
    10.
}

#[derive(Debug)]
pub struct BoxConfig {
    pub widgets: Vec<BoxedWidgetConfig>,
    pub box_conf: BoxSelf,
    pub outlook: Option<Outlook>,
}

pub fn visit_config(d: Value) -> Result<Widget, String> {
    if !d.is_object() {
        return Err("Box must be object".to_string());
    }

    let widgets = {
        let ws = d
            .get("widgets")
            .unwrap_or(&Value::Array(vec![]))
            .as_array()
            .ok_or("Widgets must be array")?
            .clone();
        ws.into_iter()
            .map(|v| {
                if !v.is_object() {
                    return Err("Widget must be object".to_string());
                }
                let index = {
                    let v = v.get("index").ok_or("index must be specified")?.clone();
                    from_value::<[isize; 2]>(v)?
                };
                let widget = {
                    let wv = v.get("widget").ok_or("widget must be specified")?.clone();
                    serde_jsonrc::from_value(wv).map_err(|e| format!("widget parse error {e}"))?
                };
                Ok(BoxedWidgetConfig { index, widget })
            })
            .collect::<Result<Vec<BoxedWidgetConfig>, String>>()?
    };

    let outlook = {
        const OUTLOOK_WINDOW: &str = "window";
        let obj = d
            .get("outlook")
            .unwrap_or(&Value::Object(Map::new()))
            .clone();
        let ol = {
            obj.as_object()
                .ok_or("Outlook Must be object")?
                .get("type")
                .cloned()
                .unwrap_or(Value::String(OUTLOOK_WINDOW.to_string()))
                .as_str()
                .ok_or("Outlook type must be string")?
                .to_string()
        };
        match ol.as_str() {
            OUTLOOK_WINDOW => Outlook::Window(from_value::<OutlookWindowConfig>(obj)?),
            _ => {
                return Err(format!("Invalid outlook {}", ol));
            }
        }
    };

    let box_conf = from_value::<BoxSelf>(d)?;

    Ok(Widget::WrapBox(Box::new(BoxConfig {
        widgets,
        box_conf,
        outlook: Some(outlook),
    })))
}

#[derive(Educe)]
#[educe(Debug)]
pub enum BoxedWidget {
    Ring(Box<RingConfig>),
    Text(Box<TextConfig>),
    Tray,
}

impl<'de> Deserialize<'de> for BoxedWidget {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = serde_jsonrc::value::Value::deserialize(deserializer)?;

        if !raw.is_object() {
            return Err(serde::de::Error::custom("Widget must be object"));
        }
        let t = raw
            .get("type")
            .ok_or(serde::de::Error::missing_field("type"))?
            .as_str()
            .ok_or(serde::de::Error::custom("widget type must be string"))?;

        match t {
            ring::NAME => ring::visit_config(raw),
            text::NAME => text::visit_config(raw),
            "tray" => Ok(BoxedWidget::Tray),
            _ => Err(format!("unknown widget type: {t}")),
        }
        .map_err(serde::de::Error::custom)
    }
}

pub mod common {
    use std::{borrow::Cow, str::FromStr};

    use serde::Deserialize;

    use crate::notify_send;

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
            impl<'de> serde::de::Visitor<'de> for TemplateVisiter {
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
