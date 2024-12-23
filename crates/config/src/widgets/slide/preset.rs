use educe::Educe;
use gtk::gdk::RGBA;
use serde::{Deserialize, Deserializer};
use util::shell::shell_cmd;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Preset {
    Speaker(PulseAudioConfig),
    Microphone(PulseAudioConfig),
    Backlight(BacklightConfig),
    Custom(CustomConfig),
}

#[derive(Debug, Deserialize)]
pub struct PulseAudioConfig {
    #[serde(default)]
    pub redraw_only_on_pa_change: bool,
    #[serde(default = "default_mute_color")]
    #[serde(deserialize_with = "super::common::color_translate")]
    pub mute_color: RGBA,
    pub device: Option<String>,
}

fn default_mute_color() -> RGBA {
    RGBA::BLACK
}

#[derive(Debug, Deserialize)]
pub struct BacklightConfig {
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub redraw_only_on_change: bool,
}

pub type UpdateTask = Box<dyn Send + Sync + FnMut() -> Result<f64, String>>;

#[derive(Educe, Deserialize)]
#[educe(Debug)]
pub struct CustomConfig {
    #[educe(Debug(ignore))]
    #[serde(default)]
    #[serde(deserialize_with = "update_task_interval")]
    pub update_with_interval_ms: Option<(u64, UpdateTask)>,
}

pub fn update_task_interval<'de, D>(d: D) -> Result<Option<(u64, UpdateTask)>, D::Error>
where
    D: Deserializer<'de>,
{
    struct EventMapVisitor;
    impl<'de> serde::de::Visitor<'de> for EventMapVisitor {
        type Value = Option<(u64, UpdateTask)>;

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let ms = seq.next_element()?.unwrap();
            let ut = seq.next_element()?.unwrap();
            Ok(Some((ms, create_update_task(ut))))
        }

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("vec of tuples: (key: number, command: string)")
        }
    }
    d.deserialize_any(EventMapVisitor)
}
pub fn create_update_task(value: String) -> UpdateTask {
    Box::new(move || {
        use std::str::FromStr;
        let a = shell_cmd(&value)?;
        f64::from_str(a.trim()).map_err(|e| format!("Fail to convert result({a}) to f64: {e}"))
    })
}
