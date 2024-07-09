use educe::Educe;
use gtk::gdk::RGBA;
use serde::{Deserialize, Deserializer};
use serde_jsonrc::Value;
use way_edges_derive::GetSize;

use crate::config::{widgets::common::create_task, NumOrRelative, Widget};

use super::common::{self, Task};

#[derive(Clone, Copy, Debug, Deserialize, Default)]
pub enum Direction {
    #[default]
    Forward,
    Backward,
}

#[derive(Educe, Deserialize, GetSize)]
#[educe(Debug)]
pub struct SlideConfig {
    pub width: NumOrRelative,
    pub height: NumOrRelative,

    #[serde(default = "common::dt_transition_duration")]
    pub transition_duration: u64,
    #[serde(default = "common::dt_frame_rate")]
    pub frame_rate: u32,
    #[serde(default = "common::dt_extra_trigger_size")]
    pub extra_trigger_size: NumOrRelative,

    #[serde(deserialize_with = "common::color_translate")]
    pub bg_color: RGBA,
    #[serde(deserialize_with = "common::color_translate")]
    pub fg_color: RGBA,
    #[serde(deserialize_with = "common::color_translate")]
    pub border_color: RGBA,
    #[serde(deserialize_with = "common::color_translate")]
    pub text_color: RGBA,
    #[serde(default)]
    pub is_text_position_start: bool,
    #[serde(default = "dt_preview_size")]
    pub preview_size: f64,
    #[serde(default)]
    pub progress_direction: Direction,
    #[educe(Debug(ignore))]
    #[serde(deserialize_with = "on_change_translate")]
    pub on_change: Task,
}

fn dt_preview_size() -> f64 {
    3.
}

pub fn visit_slide_config(d: Value) -> Result<Widget, String> {
    let c = serde_jsonrc::from_value::<SlideConfig>(d)
        .map_err(|e| format!("Fail to parse btn config: {}", e))?;
    Ok(Widget::Slider(Box::new(c)))
}

pub fn on_change_translate<'de, D>(d: D) -> Result<Task, D::Error>
where
    D: Deserializer<'de>,
{
    struct EventMapVisitor;
    impl<'de> serde::de::Visitor<'de> for EventMapVisitor {
        type Value = Task;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("vec of tuples: (key: number, command: string)")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            self.visit_string(v.to_string())
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(create_task(v))
        }
    }
    d.deserialize_any(EventMapVisitor)
}
