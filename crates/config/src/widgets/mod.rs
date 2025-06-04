use button::BtnConfig;
use schemars::JsonSchema;
use serde::Deserialize;
use slide::base::SlideConfig;
use workspace::WorkspaceConfig;
use wrapbox::BoxConfig;

pub mod button;
pub mod slide;
pub mod workspace;
pub mod wrapbox;

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum Widget {
    Btn(BtnConfig),
    Slider(SlideConfig),
    WrapBox(BoxConfig),
    Workspace(WorkspaceConfig),
}

pub mod common {
    use std::collections::HashMap;

    use cosmic_text::Color;
    use schemars::JsonSchema;
    use serde::{self, Deserialize, Deserializer, Serialize};
    use serde_jsonrc::Value;
    use smithay_client_toolkit::shell::wlr_layer::Anchor;
    use util::{color::parse_color, shell::shell_cmd_non_block};

    #[derive(Debug, Deserialize, JsonSchema, Clone)]
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

    #[derive(Debug, Default, JsonSchema, Clone)]
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
            struct EventMapVisitor;
            impl<'a> serde::de::Visitor<'a> for EventMapVisitor {
                type Value = KeyEventMap;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("vec of tuples: (key: number, command: string)")
                }

                fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                where
                    A: serde::de::MapAccess<'a>,
                {
                    let mut event_map = HashMap::new();
                    while let Some((key, value)) = map.next_entry::<String, String>()? {
                        event_map.insert(key.parse().map_err(serde::de::Error::custom)?, value);
                    }
                    Ok(KeyEventMap(event_map))
                }
            }
            deserializer.deserialize_any(EventMapVisitor)
        }
    }

    pub fn option_color_translate<'de, D>(d: D) -> Result<Option<Color>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ColorVisitor;
        impl serde::de::Visitor<'_> for ColorVisitor {
            type Value = Option<Color>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("A string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Some(parse_color(v).map_err(serde::de::Error::custom)?))
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

    pub fn color_translate<'de, D>(d: D) -> Result<Color, D::Error>
    where
        D: Deserializer<'de>,
    {
        if let Some(c) = option_color_translate(d)? {
            Ok(c)
        } else {
            Err(serde::de::Error::missing_field("color is not optional"))
        }
    }

    pub fn from_value<T>(v: Value) -> Result<T, String>
    where
        T: serde::de::DeserializeOwned,
    {
        serde_jsonrc::from_value::<T>(v).map_err(|e| format!("Fail to parse config: {e}"))
    }

    pub fn schema_color(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "default": "#00000000",
        })
    }
    pub fn schema_optional_color(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": ["string", "null"],
            "default": "#00000000",
        })
    }
    pub fn schema_template(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "default": "{float:2,100}",
        })
    }
    pub fn schema_optional_template(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": ["string", "null"],
            "default": "{float:2,100}",
        })
    }

    use cosmic_text::FamilyOwned;

    use crate::shared::NumOrRelative;

    #[derive(Serialize, Deserialize)]
    #[serde(remote = "FamilyOwned")]
    #[serde(rename_all = "kebab-case")]
    pub enum FamilyOwnedRef {
        Serif,
        SansSerif,
        Cursive,
        Fantasy,
        Monospace,
        #[serde(untagged)]
        Name(
            #[serde(deserialize_with = "deserialize_smol_str")]
            #[serde(serialize_with = "serialize_smol_str")]
            smol_str::SmolStr,
        ),
    }

    impl JsonSchema for FamilyOwnedRef {
        fn schema_name() -> std::borrow::Cow<'static, str> {
            "FamilyOwned".into()
        }

        fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
            schemars::json_schema!({
                "oneOf": [
                    {
                        "enum": [
                            "serif",
                            "sans-serif",
                            "cursive",
                            "fantasy",
                            "monospace",
                        ],
                    },
                    {
                        "type": "string",
                    }
                ],
            })
        }
    }

    pub fn dt_family_owned() -> FamilyOwned {
        FamilyOwned::Monospace
    }

    // deserialize SmolStr
    fn deserialize_smol_str<'de, D>(d: D) -> Result<smol_str::SmolStr, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(d)?;
        Ok(s.into())
    }

    fn serialize_smol_str<S>(s: &smol_str::SmolStr, d: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        String::serialize(&s.to_string(), d)
    }
}
