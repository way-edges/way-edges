use cosmic_text::Color;
use knus::{Decode, DecodeScalar};
use util::color::parse_color;
use way_edges_derive::GetSize;

use crate::kdl::{
    shared::{CommonSize, Curve},
    util::{argv, argv_str, argv_v, ToKdlError},
};

#[derive(Debug, GetSize, Clone)]
pub struct WorkspaceConfig {
    pub size: CommonSize,
    pub gap: i32,
    pub active_increase: f64,
    pub workspace_transition_duration: u64,
    pub workspace_animation_curve: Curve,
    pub pop_duration: u64,
    pub default_color: Color,
    pub focus_color: Color,
    pub active_color: Color,
    pub hover_color: Option<Color>,
    pub invert_direction: bool,
    pub output_name: Option<String>,
    pub focused_only: bool,
    pub border_width: Option<i32>,
    pub border_radius: i32,
    pub preset: WorkspacePreset,
}

impl<S: knus::traits::ErrorSpan> knus::Decode<S> for WorkspaceConfig {
    fn decode_node(
        node: &knus::ast::SpannedNode<S>,
        ctx: &mut knus::decode::Context<S>,
    ) -> Result<Self, knus::errors::DecodeError<S>> {
        let size = CommonSize::decode_node(node, ctx)?;

        let mut gap = dt_gap();
        let mut active_increase = dt_active_increase();
        let mut workspace_transition_duration = dt_workspace_transition_duration();
        let mut workspace_animation_curve = Curve::default();
        let mut pop_duration = dt_pop_duration();
        let mut default_color = dt_default_color();
        let mut focus_color = dt_focus_color();
        let mut active_color = dt_active_color();
        let mut hover_color = None;
        let mut invert_direction = false;
        let mut output_name = None;
        let mut focused_only = false;
        let mut border_width = None;
        let mut border_radius = dt_border_radius();

        let mut preset = WorkspacePreset::Hyprland;

        for child in node.children() {
            match child.node_name.as_ref() {
                "gap" => {
                    gap = argv_v(child, ctx)?;
                }
                "active-increase" => {
                    active_increase = argv_v(child, ctx)?;
                }
                "workspace-transition-duration" => {
                    workspace_transition_duration = argv_v(child, ctx)?;
                }
                "workspace-animation-curve" => {
                    workspace_animation_curve = Curve::decode(argv(child)?, ctx)?;
                }
                "pop-duration" => {
                    pop_duration = argv_v(child, ctx)?;
                }
                "default-color" => {
                    default_color = parse_color(&argv_str(child, ctx)?).to_kdl_error(child)?;
                }
                "focus-color" => {
                    focus_color = parse_color(&argv_str(child, ctx)?).to_kdl_error(child)?;
                }
                "active-color" => {
                    active_color = parse_color(&argv_str(child, ctx)?).to_kdl_error(child)?;
                }
                "hover-color" => {
                    hover_color = Some(parse_color(&argv_str(child, ctx)?).to_kdl_error(child)?);
                }
                "invert-direction" => {
                    invert_direction = true;
                }
                "output-name" => {
                    output_name = Some(argv_str(child, ctx)?);
                }
                "focused-only" => {
                    focused_only = true;
                }
                "border-width" => {
                    border_width = Some(argv_v(child, ctx)?);
                }
                "border-radius" => {
                    border_radius = argv_v(child, ctx)?;
                }
                "preset" => {
                    preset = WorkspacePreset::decode_node(child, ctx)?;
                }
                _ => {}
            }
        }

        Ok(Self {
            size,
            gap,
            active_increase,
            workspace_transition_duration,
            workspace_animation_curve,
            pop_duration,
            default_color,
            focus_color,
            active_color,
            hover_color,
            invert_direction,
            output_name,
            focused_only,
            border_width,
            border_radius,
            preset,
        })
    }
}

fn dt_border_radius() -> i32 {
    5
}

fn dt_gap() -> i32 {
    5
}
fn dt_active_increase() -> f64 {
    0.5
}
fn dt_workspace_transition_duration() -> u64 {
    300
}
fn dt_pop_duration() -> u64 {
    1000
}

fn dt_default_color() -> Color {
    parse_color("#003049").unwrap()
}
fn dt_focus_color() -> Color {
    parse_color("#669bbc").unwrap()
}
fn dt_active_color() -> Color {
    parse_color("#aaa").unwrap()
}

#[derive(Debug, Clone)]
pub enum WorkspacePreset {
    Hyprland,
    Niri(NiriConf),
}

impl<S: knus::traits::ErrorSpan> knus::Decode<S> for WorkspacePreset {
    fn decode_node(
        node: &knus::ast::SpannedNode<S>,
        ctx: &mut knus::decode::Context<S>,
    ) -> Result<Self, knus::errors::DecodeError<S>> {
        match argv_str(node, ctx)?.as_str() {
            "hyprland" => Ok(Self::Hyprland),
            "niri" => Ok(Self::Niri(NiriConf::decode_node(node, ctx)?)),
            other => Err(knus::errors::DecodeError::unexpected(
                node,
                "preset type",
                format!("unknown workspace preset: {other}"),
            )),
        }
    }
}

#[derive(Debug, Decode, Clone)]
pub struct NiriConf {
    #[knus(child)]
    pub preserve_empty: bool,
}
