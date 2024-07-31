use std::str::FromStr;

use gtk::gdk::RGBA;
use serde::Deserialize;
use serde_jsonrc::{Map, Value};

use crate::config::{parse::parse_widget, NumOrRelative, Widget};

use super::common::{self, from_value};

pub const NAME: &str = "box";

#[derive(Debug, Deserialize)]
pub struct OutlookWindowConfig {
    #[serde(default = "dt_margins")]
    pub margins: Option<[f64; 4]>,
    #[serde(default = "dt_color")]
    #[serde(deserialize_with = "common::color_translate")]
    pub border_color: RGBA,
    #[serde(default = "dt_radius")]
    pub radius: f64,
    #[serde(default = "dt_border_width")]
    pub border_width: f64,
}
fn dt_margins() -> Option<[f64; 4]> {
    Some([5., 5., 5., 5.])
}
fn dt_color() -> RGBA {
    RGBA::from_str("#4d8080").unwrap()
}
fn dt_radius() -> f64 {
    5.
}
fn dt_border_width() -> f64 {
    15.
}

#[derive(Debug, Deserialize)]
pub enum Outlook {
    Window(OutlookWindowConfig),
}

#[derive(Debug)]
pub struct BoxedWidgetConfig {
    pub index: [isize; 2],
    pub widget: Widget,
}

#[derive(Deserialize, Debug)]
pub struct BoxSelf {
    #[serde(default = "dt_gap")]
    pub gap: f64,
    #[serde(default = "common::dt_extra_trigger_size")]
    pub extra_trigger_size: NumOrRelative,
    #[serde(default = "common::dt_transition_duration")]
    pub transition_duration: u64,
    #[serde(default = "common::dt_frame_rate")]
    pub frame_rate: u32,
}

fn dt_gap() -> f64 {
    10.
}

#[derive(Debug)]
pub struct BoxConfig {
    pub widgets: Vec<BoxedWidgetConfig>,
    pub box_conf: BoxSelf,
    pub outlook: Outlook,
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
                    let widget = parse_widget(wv)?;
                    match &widget {
                        Widget::Ring(_) => Ok(widget),
                        _ => Err("Unsupported boxed widget"),
                    }?
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
        outlook,
    })))
}
