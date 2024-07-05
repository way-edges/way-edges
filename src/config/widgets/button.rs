use crate::config::{NumOrRelative, Widget};
use educe::Educe;
use gtk::gdk::RGBA;
use serde::Deserializer;
use serde_jsonrc::Value;
use std::collections::HashMap;
use std::{process::Command, str::FromStr, thread};

pub type EventMap = HashMap<u32, Box<dyn FnMut() + Send + Sync>>;

#[derive(Educe)]
#[educe(Debug)]
pub struct BtnConfig {
    #[educe(Debug(ignore))]
    pub event_map: Option<EventMap>,
    pub color: RGBA,
    pub transition_duration: u64,
    pub frame_rate: u64,
    pub extra_trigger_size: NumOrRelative,
}

pub fn visit_btn_config<'de, D>(d: D) -> Result<Widget, D::Error>
where
    D: Deserializer<'de>,
{
    struct BtnConfigVisitor;
    impl<'de> serde::de::Visitor<'de> for BtnConfigVisitor {
        type Value = Widget;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("struct BtnConfig")
        }
        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut event_map = None;
            let mut color = None;
            let mut transition_duration = None;
            let mut frame_rate = None;
            let mut extra_trigger_size = None;
            while let Some(key) = map.next_key::<String>()? {
                match key.as_str() {
                    "event_map" => {
                        event_map = Some(event_map_translate(map.next_value()?));
                    }
                    "color" => {
                        let c = map.next_value()?;
                        color = Some(color_translate(c).map_err(serde::de::Error::custom)?);
                    }
                    "transition_duration" => {
                        transition_duration = Some(map.next_value()?);
                    }
                    "frame_rate" => {
                        let fps = map.next_value()?;
                        if fps < 1 {
                            return Err(serde::de::Error::custom("Frame rate must be >= 1"));
                        }
                        frame_rate = Some(fps);
                    }
                    "extra_trigger_size" => {
                        let v: Value = map.next_value()?;
                        let res = crate::config::transform_num_or_relative(v)
                            .map_err(serde::de::Error::custom)?;
                        extra_trigger_size = Some(res);
                    }
                    _ => {}
                };
            }
            let w = Widget::Btn(Box::new(BtnConfig {
                event_map: Some(event_map.unwrap_or_default()),
                color: color.unwrap_or(RGBA::from_str("#7B98FF").unwrap()),
                transition_duration: transition_duration.unwrap_or(100),
                frame_rate: frame_rate.unwrap_or(60),
                extra_trigger_size: extra_trigger_size.unwrap_or(NumOrRelative::Num(5.)),
            }));
            Ok(w)
        }
    }
    d.deserialize_any(BtnConfigVisitor)
}

fn color_translate(color: String) -> Result<RGBA, String> {
    match RGBA::from_str(&color) {
        Ok(c) => Ok(c),
        Err(e) => Err(format!("invalid color {}", e)),
    }
}

fn event_map_translate(event_map: Vec<(u32, String)>) -> EventMap {
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
