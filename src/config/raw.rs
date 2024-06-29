use serde::{Deserialize, Deserializer};
use std::str::FromStr;

use super::NumOrRelative;

fn transform_num_or_relative_f64<'de, D>(d: D) -> Result<NumOrRelative<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    struct F64OrRelativeVisitor;
    impl<'de> serde::de::Visitor<'de> for F64OrRelativeVisitor {
        type Value = NumOrRelative<f64>;

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
            let re = regex::Regex::new(r"^(\d+(\.\d+)?)%\s*(.*)$").unwrap();

            if let Some(captures) = re.captures(v) {
                let percentage_str = captures.get(1).map_or("", |m| m.as_str());
                let percentage = f64::from_str(percentage_str).map_err(E::custom)?;

                // // description
                // let description = captures
                //     .get(3)
                //     .map(|m| {
                //         let desc = m.as_str().trim();
                //         if desc.is_empty() {
                //             None
                //         } else {
                //             Some(desc.to_string())
                //         }
                //     })
                //     .flatten();

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

fn transform_num_or_relative_i32<'de, D>(d: D) -> Result<NumOrRelative<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    let a = transform_num_or_relative_f64(d)?;
    Ok(a.convert_i32())
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct RawMargins {
    #[serde(default)]
    #[serde(deserialize_with = "transform_num_or_relative_i32")]
    pub top: NumOrRelative<i32>,
    #[serde(default)]
    #[serde(deserialize_with = "transform_num_or_relative_i32")]
    pub left: NumOrRelative<i32>,
    #[serde(default)]
    #[serde(deserialize_with = "transform_num_or_relative_i32")]
    pub right: NumOrRelative<i32>,
    #[serde(default)]
    #[serde(deserialize_with = "transform_num_or_relative_i32")]
    pub bottom: NumOrRelative<i32>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct RawConfig {
    #[serde(default = "dt_edge")]
    pub edge: String,
    #[serde(default)]
    pub position: String,
    #[serde(default = "dt_layer")]
    pub layer: String,
    #[serde(default)]
    #[serde(deserialize_with = "transform_num_or_relative_f64")]
    pub width: NumOrRelative<f64>,
    #[serde(default)]
    #[serde(deserialize_with = "transform_num_or_relative_f64")]
    pub height: NumOrRelative<f64>,
    #[serde(default)]
    pub event_map: Vec<(u32, String)>,
    #[serde(default = "dt_color")]
    pub color: String,
    #[serde(default = "dt_duration")]
    pub transition_duration: u64,
    #[serde(default = "dt_frame_rate")]
    pub frame_rate: u64,
    #[serde(default = "dt_trigger_size")]
    #[serde(deserialize_with = "transform_num_or_relative_i32")]
    pub extra_trigger_size: NumOrRelative<i32>,
    #[serde(default)]
    pub monitor_id: usize,
    #[serde(default)]
    pub monitor_name: String,
    #[serde(default)]
    pub margin: RawMargins,
}
fn dt_edge() -> String {
    String::from("left")
}
fn dt_layer() -> String {
    String::from("top")
}
fn dt_color() -> String {
    String::from("#7B98FF")
}
fn dt_duration() -> u64 {
    100
}
fn dt_frame_rate() -> u64 {
    30
}
fn dt_trigger_size() -> NumOrRelative<i32> {
    NumOrRelative::Num(5)
}

#[derive(Deserialize, Debug)]
pub struct RawGroup {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub widgets: Vec<RawConfig>,
}
#[derive(Deserialize, Debug)]
pub struct RawTemp {
    #[serde(default)]
    pub groups: Vec<RawGroup>,
}
