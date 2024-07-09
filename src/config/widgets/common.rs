use std::str::FromStr;

use gtk::gdk::RGBA;
use serde::{self, Deserializer};

use crate::config::NumOrRelative;

pub const DEFAULT_TRANSITION_DURATION: u64 = 100;
pub const DEFAULT_FRAME_RATE: u32 = 60;
pub const DEFAULT_EXTRA_TRIGGER_SIZE: NumOrRelative = NumOrRelative::Num(5.0);

pub fn dt_transition_duration() -> u64 {
    DEFAULT_TRANSITION_DURATION
}

pub fn dt_frame_rate() -> u32 {
    DEFAULT_FRAME_RATE
}

pub fn dt_extra_trigger_size() -> NumOrRelative {
    DEFAULT_EXTRA_TRIGGER_SIZE
}

pub fn color_translate<'de, D>(d: D) -> Result<RGBA, D::Error>
where
    D: Deserializer<'de>,
{
    struct ColorVisitor;
    impl<'de> serde::de::Visitor<'de> for ColorVisitor {
        type Value = RGBA;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("A string")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            to_color(v)
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            self.visit_str(v.as_str())
        }
    }
    d.deserialize_any(ColorVisitor)
}

pub fn to_color<T: serde::de::Error>(color: &str) -> Result<RGBA, T> {
    match RGBA::from_str(color) {
        Ok(c) => Ok(c),
        Err(e) => Err(format!("invalid color {}", e)),
    }
    .map_err(serde::de::Error::custom)
}
