use button::BtnConfig;
use hypr_workspace::HyprWorkspaceConfig;
use serde::{Deserialize, Deserializer};
use slide::base::SlideConfig;
use wrapbox::BoxConfig;

pub mod button;
pub mod hypr_workspace;
pub mod slide;
pub mod wrapbox;

#[derive(Debug)]
pub enum Widget {
    Btn(Box<BtnConfig>),
    Slider(Box<SlideConfig>),
    WrapBox(Box<BoxConfig>),
    HyprWorkspace(Box<HyprWorkspaceConfig>),
}

macro_rules! match_widget {
    ($t:expr, $raw:expr, $($name:ident),*) => {
        match $t {
            $(
                $name::NAME => $name::visit_config($raw),
            )*
            _ => Err(format!("unknown widget type: {}", $t)),
        }.map_err(serde::de::Error::custom)
    };
}
pub(crate) use match_widget;

impl<'de> Deserialize<'de> for Widget {
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

        match_widget!(t, raw, button, slide, wrapbox, hypr_workspace)
    }
}

pub mod common {
    use std::{collections::HashMap, str::FromStr};

    use gtk::gdk::RGBA;
    use gtk4_layer_shell::Edge;
    use serde::{self, Deserialize, Deserializer};
    use serde_jsonrc::Value;

    use crate::common::NumOrRelative;

    #[derive(Debug, Deserialize)]
    pub struct CommonSize {
        pub thickness: NumOrRelative,
        pub length: NumOrRelative,
    }
    impl CommonSize {
        pub fn calculate_relative(&mut self, monitor_size: (i32, i32), edge: Edge) {
            let max_size = match edge {
                Edge::Left | Edge::Right => (monitor_size.0, monitor_size.1),
                Edge::Top | Edge::Bottom => (monitor_size.1, monitor_size.0),
                _ => unreachable!(),
            };
            self.thickness.calculate_relative(max_size.0 as f64);
            self.length.calculate_relative(max_size.1 as f64);
        }
    }

    pub type EventMap = HashMap<u32, Task>;
    pub type Task = Box<dyn FnMut() + Send + Sync>;

    pub fn create_task(value: String) -> Task {
        use util::shell::shell_cmd_non_block;
        Box::new(move || {
            shell_cmd_non_block(value.clone());
        })
    }

    pub fn dt_event_map() -> EventMap {
        EventMap::new()
    }

    pub fn event_map_translate<'de, D>(d: D) -> Result<EventMap, D::Error>
    where
        D: Deserializer<'de>,
    {
        fn translate(event_map: Vec<(u32, String)>) -> EventMap {
            let mut map = EventMap::new();
            for (key, value) in event_map {
                map.insert(key, create_task(value));
            }
            map
        }
        struct EventMapVisitor;
        impl<'de> serde::de::Visitor<'de> for EventMapVisitor {
            type Value = EventMap;

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
                Ok(translate(event_map))
            }
        }
        d.deserialize_any(EventMapVisitor)
    }

    pub fn option_color_translate<'de, D>(d: D) -> Result<Option<RGBA>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ColorVisitor;
        impl serde::de::Visitor<'_> for ColorVisitor {
            type Value = Option<RGBA>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("A string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Some(to_color(v)?))
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

    pub fn color_translate<'de, D>(d: D) -> Result<RGBA, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ColorVisitor;
        impl serde::de::Visitor<'_> for ColorVisitor {
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
}
