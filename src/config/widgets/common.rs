use std::{collections::HashMap, str::FromStr};

use gtk::gdk::RGBA;
use serde::{self, Deserializer};
use serde_jsonrc::Value;

use crate::{config::NumOrRelative, plug::common::shell_cmd_non_block};

pub type EventMap = HashMap<u32, Task>;
pub type Task = Box<dyn FnMut() + Send + Sync>;

pub fn create_task(value: String) -> Task {
    Box::new(move || {
        shell_cmd_non_block(value.clone());
    })
}

pub fn dt_transition_duration() -> u64 {
    100
}

pub fn dt_frame_rate() -> u32 {
    60
}

pub fn dt_extra_trigger_size() -> NumOrRelative {
    NumOrRelative::Num(5.0)
}

pub fn dt_event_map() -> Option<EventMap> {
    Some(EventMap::new())
}

pub fn event_map_translate<'de, D>(d: D) -> Result<Option<EventMap>, D::Error>
where
    D: Deserializer<'de>,
{
    fn _event_map_translate(event_map: Vec<(u32, String)>) -> EventMap {
        let mut map = EventMap::new();
        for (key, value) in event_map {
            map.insert(key, create_task(value));
        }
        map
    }
    struct EventMapVisitor;
    impl<'de> serde::de::Visitor<'de> for EventMapVisitor {
        type Value = Option<EventMap>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("vec of tuples: (key: number, command: string)")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut event_map = Vec::new();
            while let Some(v) = seq.next_element::<(u32, String)>()? {
                event_map.push(v);
            }
            Ok(Some(_event_map_translate(event_map)))
        }
    }
    d.deserialize_any(EventMapVisitor)
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

pub fn from_value<T>(v: Value) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    serde_jsonrc::from_value::<T>(v).map_err(|e| format!("Fail to parse config: {e}"))
}
