use cosmic_text::{Color, FamilyOwned};
use knus::{errors::DecodeError, Decode, DecodeScalar};
use regex_lite::Regex;
use schemars::{json_schema, JsonSchema};
use serde::{Deserialize, Deserializer};
use serde_jsonrc::Value;
use smithay_client_toolkit::shell::wlr_layer::Anchor;
use std::collections::HashMap;
use std::convert::Infallible;
use std::ops::Deref;
use std::str::FromStr;
use string_to_num::ParseNum;
use util::color::parse_color;
use util::shell::shell_cmd_non_block;

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

#[derive(Debug, Clone, Copy, Default, DecodeScalar, PartialEq, Deserialize, JsonSchema)]
pub enum Curve {
    Linear,
    EaseQuad,
    #[default]
    EaseCubic,
    EaseExpo,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumOrRelative {
    Num(f64),
    Relative(f64),
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
impl<S: knus::traits::ErrorSpan> knus::DecodeScalar<S> for NumOrRelative {
    fn type_check(
        type_name: &Option<knus::span::Spanned<knus::ast::TypeName, S>>,
        ctx: &mut knus::decode::Context<S>,
    ) {
        if let Some(type_name) = &type_name {
            ctx.emit_error(DecodeError::unexpected(
                type_name,
                "type name",
                "no type name expected for this node",
            ));
        }
    }

    fn raw_decode(
        val: &knus::span::Spanned<knus::ast::Literal, S>,
        ctx: &mut knus::decode::Context<S>,
    ) -> Result<NumOrRelative, DecodeError<S>> {
        match &**val {
            knus::ast::Literal::String(ref s) => {
                // just `unwrap`, it's ok
                lazy_static::lazy_static! {
                    static ref re: Regex = Regex::new(r"^(\d+(\.\d+)?)%\s*(.*)$").unwrap();
                }

                if let Some(captures) = re.captures(s) {
                    let percentage_str = captures.get(1).map_or("", |m| m.as_str());
                    let percentage = f64::from_str(percentage_str)
                        .map_err(|e| DecodeError::conversion(val, e))?;

                    Ok(NumOrRelative::Relative(percentage * 0.01))
                } else {
                    Err(DecodeError::unsupported(
                        val,
                        "Input does not match the expected format.".to_string(),
                    ))
                }
            }
            knus::ast::Literal::Decimal(ref value) => match value.try_into() {
                Ok(v) => Ok(NumOrRelative::Num(v)),
                Err(e) => Err(DecodeError::conversion(val, e)),
            },
            knus::ast::Literal::Int(ref value) => match TryInto::<isize>::try_into(value) {
                Ok(v) => Ok(NumOrRelative::Num(v as f64)),
                Err(e) => Err(DecodeError::conversion(val, e)),
            },
            _ => Err(DecodeError::unsupported(
                val,
                "Unsupported value, only numbers and strings are recognized",
            )),
        }
    }
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

#[derive(Debug, Clone, Default, Decode, PartialEq, Deserialize, JsonSchema)]
pub struct CommonSize {
    #[knus(child, unwrap(argument))]
    pub thickness: NumOrRelative,
    #[knus(child, unwrap(argument))]
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
impl Deref for KeyEventMap {
    type Target = HashMap<u32, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<S: knus::traits::ErrorSpan> knus::Decode<S> for KeyEventMap {
    fn decode_node(
        node: &knus::ast::SpannedNode<S>,
        ctx: &mut knus::decode::Context<S>,
    ) -> Result<Self, DecodeError<S>> {
        let mut map: HashMap<u32, String> = HashMap::new();

        for child in node.children() {
            let kc = if child.node_name.starts_with("kc-") {
                // strip the "kc-" prefix
                let key_code_str = &child.node_name[3..];
                println!("key_code_str: {}", key_code_str);
                if let Ok(key_code) = key_code_str.parse_num::<u32>() {
                    key_code
                } else {
                    return Err(DecodeError::unsupported(
                        &child.node_name,
                        "Invalid key code after 'kc-' prefix",
                    ));
                }
            } else if let Some(kc) = ACTION_CODE_PAIRS
                .iter()
                .find_map(|&(k, code)| (k == child.node_name.as_ref()).then_some(code))
            {
                kc
            } else {
                return Err(DecodeError::unsupported(
                    &child.node_name,
                    "Unknown action key, expected 'kc_<number>' or predefined action",
                ));
            };

            let command = if let Some(arg) = child.arguments.first() {
                if let knus::ast::Literal::String(s) = arg.literal.deref() {
                    s.to_string()
                } else {
                    return Err(DecodeError::unsupported(
                        &arg.literal,
                        "Expected a string literal for command",
                    ));
                }
            } else {
                return Err(DecodeError::unexpected(
                    &child.node_name,
                    "command",
                    "Expected at least one argument for command",
                ));
            };

            map.insert(kc, command);
        }
        Ok(KeyEventMap(map))
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
                    let kc = if let Some(key_code_str) = key.strip_prefix("kc-") {
                        // strip the "kc-" prefix
                        println!("key_code_str: {}", key_code_str);
                        if let Ok(key_code) = key_code_str.parse_num::<u32>() {
                            key_code
                        } else {
                            return Err(serde::de::Error::custom(format!(
                                "Invalid key code after 'kc-' prefix: '{}'",
                                key_code_str
                            )));
                        }
                    } else if let Some(kc) = ACTION_CODE_PAIRS
                        .iter()
                        .find_map(|&(k, code)| (k == key).then_some(code))
                    {
                        kc
                    } else {
                        return Err(serde::de::Error::custom(format!(
                            "Unknown action key: '{}'.",
                            key
                        )));
                    };

                    event_map.insert(kc, value);
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_jsonrc::json;

    #[test]
    fn test_key_event_map_deserialize_valid() {
        let json_data = json!({
            "kc-0x110": "command1",
            "kc-0x112": "command2",
            "mouse-right": "command3"
        });

        let key_event_map: KeyEventMap = serde_jsonrc::from_value(json_data).unwrap();

        assert_eq!(key_event_map.0.get(&0x110).unwrap(), "command1");
        assert_eq!(key_event_map.0.get(&0x112).unwrap(), "command2");
        assert_eq!(key_event_map.0.get(&0x111).unwrap(), "command3");
    }

    #[test]
    fn test_key_event_map_deserialize_invalid_key() {
        let json_data = json!({
            "invalid-key": "command1"
        });

        let result: Result<KeyEventMap, _> = serde_jsonrc::from_value(json_data);

        // Expecting an error because the key format is invalid
        assert!(result.is_err());
    }

    #[test]
    fn test_key_event_map_deserialize_invalid_value() {
        let json_data = json!({
            "kc-110": 12345
        });

        let result: Result<KeyEventMap, _> = serde_jsonrc::from_value(json_data);

        // Expecting an error because the value is not a string
        assert!(result.is_err());
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

pub fn dt_family_owned() -> FamilyOwned {
    FamilyOwned::Monospace
}

pub fn parse_family_owned(s: &str) -> Result<FamilyOwned, Infallible> {
    Ok(match s {
        "serif" => FamilyOwned::Serif,
        "sans-serif" => FamilyOwned::SansSerif,
        "cursive" => FamilyOwned::Cursive,
        "fantasy" => FamilyOwned::Fantasy,
        "monospace" => FamilyOwned::Monospace,
        other => FamilyOwned::Name(other.into()),
    })
}

pub fn deserialize_family_owned<'de, D>(d: D) -> Result<FamilyOwned, D::Error>
where
    D: Deserializer<'de>,
{
    struct FamilyOwnedVisitor;
    impl<'a> serde::de::Visitor<'a> for FamilyOwnedVisitor {
        type Value = FamilyOwned;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string representing a font family")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            parse_family_owned(v).map_err(E::custom)
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            self.visit_str(v.as_str())
        }
    }
    d.deserialize_any(FamilyOwnedVisitor)
}

pub fn schema_family_owned(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": "string",
        "enum": ["serif", "sans-serif", "cursive", "fantasy", "monospace"],
        "default": "monospace",
    })
}

#[derive(Debug, Clone, Decode, PartialEq, Deserialize, JsonSchema)]
pub struct NumMargins {
    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub left: i32,
    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub top: i32,
    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub right: i32,
    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub bottom: i32,
}
