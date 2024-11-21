use std::str::FromStr;

use educe::Educe;
use gtk::gdk::RGBA;
use serde::{Deserialize, Deserializer};
use serde_jsonrc::Value;
use way_edges_derive::GetSize;

use crate::{
    config::{NumOrRelative, Widget},
    plug::common::{shell_cmd, shell_cmd_non_block},
};

use super::common::{self, from_value, CommonSize, EventMap};

pub const NAME: &str = "slide";

pub type Task = Box<dyn Send + Sync + FnMut(f64) -> bool>;
pub type UpdateTask = Box<dyn Send + Sync + FnMut() -> Result<f64, String>>;

#[derive(Clone, Copy, Debug, Deserialize, Default)]
pub enum Direction {
    #[default]
    Forward,
    Backward,
}

// TODO: serde_valid
#[derive(Educe, Deserialize, GetSize)]
#[educe(Debug)]
pub struct SlideConfig {
    // draw related
    #[serde(flatten)]
    pub size: CommonSize,

    #[serde(default = "common::dt_transition_duration")]
    pub transition_duration: u64,
    #[serde(default = "common::dt_frame_rate")]
    pub frame_rate: u32,
    #[serde(default = "common::dt_extra_trigger_size")]
    pub extra_trigger_size: NumOrRelative,

    #[serde(default = "dt_obtuse_angle")]
    pub obtuse_angle: f64,
    #[serde(default = "dt_radius")]
    pub radius: f64,

    #[serde(default = "dt_bg_color")]
    #[serde(deserialize_with = "common::color_translate")]
    pub bg_color: RGBA,
    #[serde(default = "dt_fg_color")]
    #[serde(deserialize_with = "common::color_translate")]
    pub fg_color: RGBA,
    #[serde(default = "dt_border_color")]
    #[serde(deserialize_with = "common::color_translate")]
    pub border_color: RGBA,
    #[serde(default = "dt_text_color")]
    #[serde(deserialize_with = "common::color_translate")]
    pub text_color: RGBA,
    #[serde(default)]
    pub is_text_position_start: bool,
    #[serde(default = "dt_preview_size")]
    pub preview_size: f64,
    #[serde(default)]
    pub progress_direction: Direction,

    // event related
    #[serde(default = "dt_draggable")]
    pub draggable: bool,

    #[educe(Debug(ignore))]
    #[serde(default)]
    #[serde(deserialize_with = "on_change_translate")]
    pub on_change: Option<Task>,

    #[educe(Debug(ignore))]
    #[serde(default)]
    #[serde(deserialize_with = "update_task_interval")]
    pub update_with_interval_ms: Option<(u64, UpdateTask)>,

    #[educe(Debug(ignore))]
    #[serde(default = "common::dt_event_map")]
    #[serde(deserialize_with = "common::event_map_translate")]
    pub event_map: Option<EventMap>,
}

fn dt_bg_color() -> RGBA {
    RGBA::from_str("#808080").unwrap()
}
fn dt_fg_color() -> RGBA {
    RGBA::from_str("#FFB847").unwrap()
}
fn dt_border_color() -> RGBA {
    RGBA::from_str("#646464").unwrap()
}
fn dt_text_color() -> RGBA {
    RGBA::from_str("#000000").unwrap()
}
fn dt_preview_size() -> f64 {
    3.
}
fn dt_obtuse_angle() -> f64 {
    120.
}
fn dt_radius() -> f64 {
    20.
}

fn dt_draggable() -> bool {
    true
}

pub fn visit_config(d: Value) -> Result<Widget, String> {
    let mut c = from_value::<SlideConfig>(d)?;

    // remove mouse event for primary and middle button
    // as for `change progress` and `pin widget`
    {
        let em = c.event_map.as_mut().unwrap();
        em.remove_entry(&1);
        em.remove_entry(&2);
    };
    Ok(Widget::Slider(Box::new(c)))
}

pub fn on_change_translate<'de, D>(d: D) -> Result<Option<Task>, D::Error>
where
    D: Deserializer<'de>,
{
    struct EventMapVisitor;
    impl<'de> serde::de::Visitor<'de> for EventMapVisitor {
        type Value = Option<Task>;

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
            Ok(Some(create_task(v)))
        }
    }
    d.deserialize_any(EventMapVisitor)
}

pub fn create_task(value: String) -> Task {
    Box::new(move |progress| {
        let value = value
            .clone()
            .replace("{progress}", progress.to_string().as_str());
        shell_cmd_non_block(value);
        true
    })
}

pub fn update_task_interval<'de, D>(d: D) -> Result<Option<(u64, UpdateTask)>, D::Error>
where
    D: Deserializer<'de>,
{
    struct EventMapVisitor;
    impl<'de> serde::de::Visitor<'de> for EventMapVisitor {
        type Value = Option<(u64, UpdateTask)>;

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
pub fn create_update_task(value: String) -> UpdateTask {
    Box::new(move || {
        let value = value.clone();
        let a = shell_cmd(value)?;
        f64::from_str(&a).map_err(|e| format!("Fail to convert result({a}) to f64: {e}"))
    })
}
