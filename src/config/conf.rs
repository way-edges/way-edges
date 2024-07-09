use crate::activate::MonitorSpecifier;
use educe::Educe;
use gtk4_layer_shell::{Edge, Layer};
use serde::Deserialize;
use std::str::FromStr;

use super::widgets::{button::BtnConfig, slide::SlideConfig};

pub type GroupConfig = Vec<Config>;

#[derive(Debug, Clone, Copy)]
pub enum NumOrRelative {
    Num(f64),
    Relative(f64),
}
impl Default for NumOrRelative {
    fn default() -> Self {
        Self::Num(f64::default())
    }
}
impl<'de> Deserialize<'de> for NumOrRelative {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct F64OrRelativeVisitor;
        impl<'de> serde::de::Visitor<'de> for F64OrRelativeVisitor {
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
}

#[allow(dead_code)]
impl NumOrRelative {
    pub fn get_num(&self) -> Result<f64, &str> {
        if let Self::Num(r) = self {
            Ok(*r)
        } else {
            println!("{self:?}");
            Err("relative, not num")
        }
    }
    pub fn get_num_into(self) -> Result<f64, &'static str> {
        // println!("hrere");
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

#[derive(Educe)]
#[educe(Debug)]
pub enum Widget {
    Btn(Box<BtnConfig>),
    ToggleBtn,
    Slider(Box<SlideConfig>),
    Combo,
    SpinBtn,
}

#[derive(Educe)]
#[educe(Debug)]
pub struct Config {
    pub edge: Edge,
    pub position: Option<Edge>,
    pub layer: Layer,
    pub monitor: MonitorSpecifier,
    pub margins: Vec<(Edge, NumOrRelative)>,

    pub widget: Option<Widget>,
    // #[educe(Debug(ignore))]
    // pub event_map: Option<EventMap>,
    // pub color: RGBA,
    // pub transition_duration: u64,
    // pub frame_rate: u64,
    // pub extra_trigger_size: NumOrRelative<i32>,
}

impl Drop for Config {
    fn drop(&mut self) {
        log::debug!("dropping config: {self:?}")
    }
}
