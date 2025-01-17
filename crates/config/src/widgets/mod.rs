use button::BtnConfig;
use hypr_workspace::HyprWorkspaceConfig;
use serde::Deserialize;
use slide::base::SlideConfig;
use wrapbox::BoxConfig;

pub mod button;
pub mod hypr_workspace;
pub mod slide;
pub mod wrapbox;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum Widget {
    Btn(BtnConfig),
    Slider(SlideConfig),
    WrapBox(BoxConfig),
    HyprWorkspace(HyprWorkspaceConfig),
}

// macro_rules! match_widget {
//     ($t:expr, $raw:expr, $($name:ident),*) => {
//         match $t {
//             $(
//                 $name::NAME => $name::visit_config($raw),
//             )*
//             _ => Err(format!("unknown widget type: {}", $t)),
//         }.map_err(serde::de::Error::custom)
//     };
// }
// pub(crate) use match_widget;
//
// impl<'de> Deserialize<'de> for Widget {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         let raw = serde_jsonrc::value::Value::deserialize(deserializer)?;
//
//         if !raw.is_object() {
//             return Err(serde::de::Error::custom("Widget must be object"));
//         }
//         let t = raw
//             .get("type")
//             .ok_or(serde::de::Error::missing_field("type"))?
//             .as_str()
//             .ok_or(serde::de::Error::custom("widget type must be string"))?;
//
//         match_widget!(t, raw, button, slide, wrapbox, hypr_workspace)
//     }
// }

pub mod common {
    use std::{collections::HashMap, fmt::Display, str::FromStr};

    use gtk::gdk::RGBA;
    use serde::{self, de, Deserialize, Deserializer};
    use serde_jsonrc::Value;
    use smithay_client_toolkit::shell::wlr_layer::Anchor;
    use util::shell::shell_cmd_non_block;

    use crate::common::NumOrRelative;

    #[derive(Debug, Deserialize)]
    pub struct CommonSize {
        pub thickness: NumOrRelative,
        pub length: NumOrRelative,
    }
    impl CommonSize {
        pub fn calculate_relative(&mut self, monitor_size: (i32, i32), edge: Anchor) {
            let max_size = match edge {
                Anchor::LEFT | Anchor::RIGHT => (monitor_size.0, monitor_size.1),
                Anchor::TOP | Anchor::BOTTOM => (monitor_size.1, monitor_size.0),
                _ => unreachable!(),
            };
            self.thickness.calculate_relative(max_size.0 as f64);
            self.length.calculate_relative(max_size.1 as f64);
        }
    }

    #[derive(Debug, Default)]
    pub struct KeyEventMap(HashMap<u32, String>);
    impl KeyEventMap {
        pub fn call(&self, k: u32) {
            if let Some(cmd) = self.0.get(&k) {
                // PERF: SHOULE THIS BE USE OF CLONING???
                shell_cmd_non_block(cmd.clone());
            }
        }
    }
    impl<'de> Deserialize<'de> for KeyEventMap {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let map: HashMap<u32, String> = de_int_key(deserializer)?;
            Ok(KeyEventMap(map))
        }
    }
    fn de_int_key<'de, D, K, V>(deserializer: D) -> Result<HashMap<K, V>, D::Error>
    where
        D: Deserializer<'de>,
        K: Eq + std::hash::Hash + FromStr,
        K::Err: Display,
        V: Deserialize<'de>,
    {
        let string_map = <HashMap<String, V>>::deserialize(deserializer)?;
        let mut map = HashMap::with_capacity(string_map.len());
        for (s, v) in string_map {
            let k = K::from_str(&s).map_err(de::Error::custom)?;
            map.insert(k, v);
        }
        Ok(map)
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
