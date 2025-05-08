use button::BtnConfig;
use schemars::{JsonSchema, Schema};
use serde::Deserialize;
use serde_json::Value;
use slide::base::SlideConfig;
use workspace::WorkspaceConfig;
use wrapbox::BoxConfig;

pub mod button;
pub mod slide;
pub mod workspace;
pub mod wrapbox;

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", tag = "type")]
#[schemars(transform = generate_defs)]
pub enum Widget {
    Btn(BtnConfig),
    Slider(SlideConfig),
    WrapBox(BoxConfig),
    Workspace(WorkspaceConfig),
}
fn generate_defs(schema: &mut Schema) {
    println!("222");
    let root = schema.ensure_object();

    match root.get_mut("oneOf") {
        Some(Value::Array(arr)) => arr,
        _ => return,
    }
    .iter_mut()
    .for_each(|v| {
        if let Some(obj) = v.as_object_mut() {
            obj.retain(|k, _| k == "$ref")
        }
    });
}

pub mod common {
    use std::collections::HashMap;

    use cosmic_text::Color;
    use schemars::JsonSchema;
    use serde::{self, Deserialize, Deserializer};
    use serde_jsonrc::Value;
    use smithay_client_toolkit::shell::wlr_layer::Anchor;
    use util::{color::parse_color, shell::shell_cmd_non_block};

    use crate::common::NumOrRelative;

    #[derive(Debug, Deserialize, JsonSchema)]
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

    #[derive(Debug, Default, JsonSchema, Deserialize)]
    pub struct KeyEventMap(HashMap<u32, String>);
    impl KeyEventMap {
        pub fn call(&self, k: u32) {
            if let Some(cmd) = self.0.get(&k) {
                // PERF: SHOULE THIS BE USE OF CLONING???
                shell_cmd_non_block(cmd.clone());
            }
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
}
