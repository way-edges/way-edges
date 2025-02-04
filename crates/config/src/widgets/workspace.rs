use super::common::{self, CommonSize};
use cosmic_text::Color;
use serde::Deserialize;
use util::color::parse_color;
use way_edges_derive::GetSize;

#[derive(Debug, Deserialize, GetSize)]
pub struct WorkspaceConfig {
    #[serde(flatten)]
    // flatten does not support `default` yet.
    // issue: https://github.com/serde-rs/serde/issues/1626
    // PR: https://github.com/serde-rs/serde/pull/2687
    // #[serde(default = "dt_size")]
    pub size: CommonSize,

    #[serde(default = "dt_gap")]
    pub gap: i32,
    #[serde(default = "dt_active_increase")]
    pub active_increase: f64,

    #[serde(default = "dt_workspace_transition_duration")]
    pub workspace_transition_duration: u64,

    #[serde(default = "dt_pop_duration")]
    pub pop_duration: u64,

    #[serde(default = "dt_deactive_color")]
    #[serde(deserialize_with = "common::color_translate")]
    pub deactive_color: Color,
    #[serde(default = "dt_active_color")]
    #[serde(deserialize_with = "common::color_translate")]
    pub active_color: Color,
    #[serde(default)]
    #[serde(deserialize_with = "common::option_color_translate")]
    pub hover_color: Option<Color>,

    #[serde(default)]
    pub invert_direction: bool,

    #[serde(default)]
    pub output_name: Option<String>,
    pub preset: WorkspacePreset,
}

fn dt_gap() -> i32 {
    5
}
fn dt_active_increase() -> f64 {
    0.5
}
fn dt_workspace_transition_duration() -> u64 {
    100
}
fn dt_pop_duration() -> u64 {
    1000
}

fn dt_deactive_color() -> Color {
    parse_color("#003049").unwrap()
}
fn dt_active_color() -> Color {
    parse_color("#669bbc").unwrap()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspacePreset {
    Hyprland,
    Niri,
}
