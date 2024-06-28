use crate::{activate::default, ui::EventMap};
use educe::Educe;
use gtk::gdk::RGBA;
use gtk4_layer_shell::{Edge, Layer};
use std::collections::HashMap;

pub type GroupConfigMap = HashMap<String, GroupConfig>;
pub type GroupConfig = Vec<Config>;

#[derive(Debug, Clone)]
pub enum MonitorSpecifier {
    ID(usize),
    Name(String),
}

#[derive(Debug, Clone, Copy)]
pub enum NumOrRelative<T: Copy + Clone> {
    Num(T),
    Relative(f64),
}
impl<T: Copy + Clone> NumOrRelative<T> {
    pub fn get_num(&self) -> Result<T, &str> {
        if let Self::Num(r) = self {
            Ok(*r)
        } else {
            Err("relative, not num")
        }
    }
    pub fn get_num_into(self) -> Result<T, &'static str> {
        if let Self::Num(r) = self {
            Ok(r)
        } else {
            Err("relative, not num")
        }
    }
}
pub trait Convert<U: Copy + Clone> {
    fn convert(self) -> NumOrRelative<U>;
}
impl<T, U> Convert<U> for NumOrRelative<T>
where
    T: Copy + Clone + Into<U>,
    U: Copy + Clone + Into<T>,
{
    fn convert(self) -> NumOrRelative<U> {
        match self {
            NumOrRelative::Num(num) => NumOrRelative::Num(num.into()),
            NumOrRelative::Relative(rel) => NumOrRelative::Relative(rel),
        }
    }
}
impl NumOrRelative<f64> {
    pub fn convert_i32(self) -> NumOrRelative<i32> {
        match self {
            NumOrRelative::Num(num) => NumOrRelative::Num(num as i32),
            NumOrRelative::Relative(rel) => NumOrRelative::Relative(rel),
        }
    }
}
// Implement Default for NumOrRelative<T> where T: Default
impl<T> Default for NumOrRelative<T>
where
    T: Copy + Clone + Default,
{
    fn default() -> Self {
        NumOrRelative::Num(T::default())
    }
}

// // Implement a generic conversion from NumOrRelative<T> to NumOrRelative<U>
// impl<T, U> From<NumOrRelative<T>> for NumOrRelative<U>
// where
//     T: Copy + Clone + Into<U>,
//     U: Copy + Clone,
// {
//     fn from(value: NumOrRelative<T>) -> Self {
//         match value {
//             NumOrRelative::Num(num) => NumOrRelative::Num(num.into()),
//             NumOrRelative::Relative(rel) => NumOrRelative::Relative(rel),
//         }
//     }
// }

#[derive(Educe)]
#[educe(Debug)]
pub struct Config {
    pub edge: Edge,
    pub position: Option<Edge>,
    pub layer: Layer,
    // pub size: (f64, f64),
    pub width: NumOrRelative<f64>,
    pub height: NumOrRelative<f64>,

    #[educe(Debug(ignore))]
    pub event_map: Option<EventMap>,

    pub color: RGBA,
    pub transition_duration: u64,
    pub frame_rate: u64,
    // pub extra_trigger_size: f64,
    pub extra_trigger_size: NumOrRelative<f64>,
    pub monitor: MonitorSpecifier,
    // pub margins: Vec<(Edge, i32)>,
    pub margins: Vec<(Edge, NumOrRelative<i32>)>,
}

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
        println!("dropping config")
    }
}
