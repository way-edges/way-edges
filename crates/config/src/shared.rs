use cosmic_text::{Color, FamilyOwned};
use regex_lite::Regex;
use schemars::{json_schema, JsonSchema};
use serde::{self, Deserialize, Deserializer, Serialize};
use serde_jsonrc::Value;
use smithay_client_toolkit::shell::wlr_layer::Anchor;
use std::collections::HashMap;
use std::str::FromStr;
use util::{color::parse_color, shell::shell_cmd_non_block};

#[rustfmt::skip]
static ACTION_CODE_PAIRS: &[(&str, u32)] = &[
    ("mouse-left",    0x110),
    ("mouse-right",   0x111),
    ("mouse-middle",  0x112),
    ("mouse-side",    0x113),
    ("mouse-extra",   0x114),
    ("mouse-forward", 0x115),
    ("mouse-back",    0x116),
];

#[derive(Debug, Clone, Copy, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Curve {
    Linear,
    EaseQuad,
    #[default]
    EaseCubic,
    EaseExpo,
}

#[derive(Debug, Clone, Copy)]
pub enum NumOrRelative {
    Num(f64),
    Relative(f64),
}
impl JsonSchema for NumOrRelative {
    fn schema_id() -> std::borrow::Cow<'static, str> {
        Self::schema_name()
    }

    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("NumOrRelative")
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        json_schema!({
            "type": ["number", "string"],
            "anyOf": [
                {
                    "type": "number",
                    "description": "absolute number"
                },
                {
                    "type": "string",
                    "pattern": r"^(\d+(\.\d+)?)%\s*(.*)$",
                    "description": "relative number"
                }
            ]
        })
    }
}
impl Default for NumOrRelative {
    fn default() -> Self {
        Self::Num(f64::default())
    }
}
#[allow(dead_code)]
impl NumOrRelative {
    pub fn is_relative(&self) -> bool {
        match self {
            NumOrRelative::Num(_) => false,
            NumOrRelative::Relative(_) => true,
        }
    }
    pub fn is_zero(&self) -> bool {
        match self {
            NumOrRelative::Num(r) => *r == 0.,
            NumOrRelative::Relative(r) => *r == 0.,
        }
    }
    pub fn get_num(&self) -> Result<f64, &str> {
        if let Self::Num(r) = self {
            Ok(*r)
        } else {
            Err("relative, not num")
        }
    }
    pub fn get_num_into(self) -> Result<f64, &'static str> {
        if let Self::Num(r) = self {
            Ok(r)
        } else {
            Err("relative, not num")
        }
    }
    pub fn is_valid_length(&self) -> bool {
        match self {
            NumOrRelative::Num(r) => *r > f64::default(),
            NumOrRelative::Relative(r) => *r > 0.,
        }
    }
    pub fn get_rel(&self) -> Result<f64, &'static str> {
        if let Self::Relative(r) = self {
            Ok(*r)
        } else {
            Err("num, not relative")
        }
    }
    pub fn get_rel_into(self) -> Result<f64, &'static str> {
        if let Self::Relative(r) = self {
            Ok(r)
        } else {
            Err("num, not relative")
        }
    }
    pub fn calculate_relative_into(self, max: f64) -> Self {
        if let Self::Relative(r) = self {
            Self::Num(r * max)
        } else {
            self
        }
    }
    pub fn calculate_relative(&mut self, max: f64) {
        if let Self::Relative(r) = self {
            *self = Self::Num(*r * max)
        }
    }
}
impl<'de> Deserialize<'de> for NumOrRelative {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct F64OrRelativeVisitor;
        impl serde::de::Visitor<'_> for F64OrRelativeVisitor {
            type Value = NumOrRelative;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a number or a string")
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(NumOrRelative::Num(v as f64))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(NumOrRelative::Num(v as f64))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(NumOrRelative::Num(v))
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                // just `unwrap`, it's ok
                lazy_static::lazy_static! {
                    static ref re: Regex = Regex::new(r"^(\d+(\.\d+)?)%\s*(.*)$").unwrap();
                }

                if let Some(captures) = re.captures(v) {
                    let percentage_str = captures.get(1).map_or("", |m| m.as_str());
                    let percentage = f64::from_str(percentage_str).map_err(E::custom)?;

                    Ok(NumOrRelative::Relative(percentage * 0.01))
                } else {
                    Err(E::custom(
                        "Input does not match the expected format.".to_string(),
                    ))
                }
            }
        }
        d.deserialize_any(F64OrRelativeVisitor)
    }
}

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

#[derive(Debug, Default, Clone)]
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
                    let action_code = if let Ok(code) = key.parse::<u32>() {
                        code
                    } else {
                        ACTION_CODE_PAIRS
                            .iter()
                            .find_map(|&(k, code)| (k == key).then_some(code))
                            .ok_or_else(|| {
                                serde::de::Error::custom(format!("Unknown action key: '{}'.", key))
                            })?
                    };
                    event_map.insert(action_code, value);
                }
                Ok(KeyEventMap(event_map))
            }
        }
        deserializer.deserialize_any(EventMapVisitor)
    }
}

impl JsonSchema for KeyEventMap {
    fn schema_id() -> std::borrow::Cow<'static, str> {
        Self::schema_name()
    }

    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("KeyEventMap")
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let allowed_str_keys: Vec<_> = ACTION_CODE_PAIRS.iter().map(|&(k, _)| k).collect();
        let str_keys_pattern = format!("^({})$", allowed_str_keys.join("|"));

        json_schema!({
            "type": "object",
            "patternProperties": {
                r"^\d+$": {"type": "string"},
                str_keys_pattern: {"type": "string"}
            },
            "additionalProperties": false
        })
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
