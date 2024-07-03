use crate::{activate::MonitorSpecifier, ui::button::BtnConfig};
use educe::Educe;
use gtk4_layer_shell::{Edge, Layer};
// use std::collections::HashMap;

// pub type GroupConfigMap = HashMap<String, GroupConfig>;
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

#[allow(dead_code)]
impl NumOrRelative {
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

#[derive(Educe)]
#[educe(Debug)]
pub enum Widget {
    Btn(Box<BtnConfig>),
    ToggleBtn,
    Slider,
    Combo,
    SpinBtn,
}

#[derive(Educe)]
#[educe(Debug)]
pub struct Config {
    pub edge: Edge,
    pub position: Option<Edge>,
    pub layer: Layer,
    pub width: NumOrRelative,
    pub height: NumOrRelative,
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

#[allow(dead_code)]
impl Config {
    pub fn get_size(&self) -> Result<(f64, f64), &str> {
        Ok((self.width.get_num()?, self.height.get_num()?))
    }
    pub fn get_size_into(&self) -> Result<(f64, f64), &str> {
        Ok((self.width.get_num_into()?, self.height.get_num_into()?))
    }
}
impl Drop for Config {
    fn drop(&mut self) {
        log::debug!("dropping config: {self:?}")
    }
}
