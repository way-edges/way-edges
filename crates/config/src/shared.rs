use regex_lite::Regex;
use schemars::{json_schema, JsonSchema};
use serde::Deserialize;
use std::str::FromStr;

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
    fn always_inline_schema() -> bool {
        false
    }

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
