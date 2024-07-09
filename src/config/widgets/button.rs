use super::common;
use crate::config::{NumOrRelative, Widget};
use educe::Educe;
use gtk::gdk::RGBA;
use serde::{Deserialize, Deserializer};
use serde_jsonrc::Value;
use std::collections::HashMap;
use std::{process::Command, str::FromStr, thread};

pub type EventMap = HashMap<u32, Box<dyn FnMut() + Send + Sync>>;

#[derive(Educe, Deserialize)]
#[educe(Debug)]
pub struct BtnConfig {
    pub width: NumOrRelative,
    pub height: NumOrRelative,

    #[educe(Debug(ignore))]
    #[serde(default = "dt_event_map")]
    #[serde(deserialize_with = "event_map_translate")]
    pub event_map: Option<EventMap>,

    #[serde(default = "dt_color")]
    #[serde(deserialize_with = "common::color_translate")]
    pub color: RGBA,
    #[serde(default = "common::dt_transition_duration")]
    pub transition_duration: u64,
    #[serde(default = "common::dt_frame_rate")]
    pub frame_rate: u32,
    #[serde(default = "common::dt_extra_trigger_size")]
    pub extra_trigger_size: NumOrRelative,
}

fn dt_color() -> RGBA {
    RGBA::from_str("#7B98FF").unwrap()
}

fn dt_event_map() -> Option<EventMap> {
    Some(EventMap::new())
}

impl BtnConfig {
    pub fn get_size(&self) -> Result<(f64, f64), String> {
        Ok((self.width.get_num()?, self.height.get_num()?))
    }
}
pub fn visit_btn_config(d: Value) -> Result<Widget, String> {
    let c = serde_jsonrc::from_value::<BtnConfig>(d)
        .map_err(|e| format!("Fail to parse btn config: {}", e))?;
    Ok(Widget::Btn(Box::new(c)))
}

fn _event_map_translate(event_map: Vec<(u32, String)>) -> EventMap {
    let mut map = EventMap::new();
    for (key, value) in event_map {
        map.insert(
            key,
            Box::new(move || {
                let value = value.clone();
                thread::spawn(move || {
                    let mut cmd = Command::new("/bin/sh");
                    let res = cmd.arg("-c").arg(&value).output();
                    if let Err(e) = res {
                        let msg = format!("error running command: {value}\nError: {e}");
                        log::error!("{msg}");
                        crate::notify_send("Way-Edges command error", &msg, true);
                    }
                });
            }),
        );
    }
    map
}

pub fn event_map_translate<'de, D>(d: D) -> Result<Option<EventMap>, D::Error>
where
    D: Deserializer<'de>,
{
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
