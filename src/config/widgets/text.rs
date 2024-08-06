use educe::Educe;
use gtk::gdk::RGBA;
use serde::{Deserialize, Deserializer};
use serde_jsonrc::Value;

use crate::{
    config::{widgets::common, Widget},
    plug::common::shell_cmd,
};

use super::common::from_value;

pub const NAME: &str = "text";

pub type TextUpdateTask = Box<dyn Send + FnMut() -> Result<String, String>>;

#[derive(Educe, Deserialize)]
#[educe(Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum TextPreset {
    Time {
        #[serde(default = "dt_time_format")]
        format: String,
        #[serde(default)]
        time_zone: Option<String>,
    },
    Custom {
        #[educe(Debug(ignore))]
        #[serde(deserialize_with = "update_task_interval")]
        update_with_interval_ms: Option<(u64, TextUpdateTask)>,
    },
}
fn dt_time_format() -> String {
    "%Y-%m-%d %H:%M:%S".to_string()
}

#[derive(Educe, Deserialize)]
#[educe(Debug)]
pub struct TextConfig {
    #[serde(default = "dt_fg_color")]
    #[serde(deserialize_with = "common::color_translate")]
    pub fg_color: RGBA,
    #[serde(default = "dt_font_size")]
    pub font_size: i32,
    #[serde(default)]
    pub font_family: Option<String>,

    pub preset: Option<TextPreset>,
}

fn dt_fg_color() -> RGBA {
    RGBA::BLACK
}
fn dt_font_size() -> i32 {
    12
}

pub fn visit_config(v: Value) -> Result<Widget, String> {
    let conf: TextConfig = from_value(v)?;
    if conf.preset.is_none() {
        return Err("preset must be set".to_string());
    }
    Ok(Widget::Text(Box::new(conf)))
}

fn update_task_interval<'de, D>(d: D) -> Result<Option<(u64, TextUpdateTask)>, D::Error>
where
    D: Deserializer<'de>,
{
    struct EventMapVisitor;
    impl<'de> serde::de::Visitor<'de> for EventMapVisitor {
        type Value = Option<(u64, TextUpdateTask)>;

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let ms = seq.next_element()?.unwrap();
            let ut = seq.next_element()?.unwrap();
            Ok(Some((ms, create_update_task(ut))))
        }

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("vec of tuples: (key: number, command: string)")
        }
    }
    d.deserialize_any(EventMapVisitor)
}
fn create_update_task(value: String) -> TextUpdateTask {
    Box::new(move || {
        let value = value.clone();
        let a = shell_cmd(value)?;
        Ok(a)
    })
}
