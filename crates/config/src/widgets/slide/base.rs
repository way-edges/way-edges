use std::str::FromStr;

use educe::Educe;
use gtk::gdk::RGBA;
use serde::{Deserialize, Deserializer};
use way_edges_derive::GetSize;

use util::template::{
    arg::TemplateArgFloatProcesser,
    base::{Template, TemplateProcesser},
};

use super::{
    common::{self, CommonSize, EventMap},
    preset::Preset,
};

pub type SlideOnChangeFunc = Box<dyn Send + Sync + FnMut(f64) -> bool>;

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

    #[educe(Debug(ignore))]
    #[serde(default)]
    #[serde(deserialize_with = "slide_change_template")]
    pub on_change: Option<Template>,

    #[educe(Debug(ignore))]
    #[serde(default = "common::dt_event_map")]
    #[serde(deserialize_with = "common::event_map_translate")]
    pub event_map: EventMap,

    #[serde(default)]
    pub preset: Option<Preset>,
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

pub fn slide_change_template<'de, D>(d: D) -> Result<Option<Template>, D::Error>
where
    D: Deserializer<'de>,
{
    struct EventMapVisitor;
    impl serde::de::Visitor<'_> for EventMapVisitor {
        type Value = Option<Template>;

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
            Ok(Some(
                Template::create_from_str(
                    &v,
                    TemplateProcesser::new().add_processer(TemplateArgFloatProcesser),
                )
                .map_err(serde::de::Error::custom)?,
            ))
        }
    }
    d.deserialize_any(EventMapVisitor)
}
