use cosmic_text::Color;
use educe::Educe;
use util::color::parse_color;
use way_edges_derive::GetSize;

use crate::kdl::shared::CommonSize;

use super::preset::Preset;

use crate::kdl::util::{argv_str, argv_v, ToKdlError};
use knus::Decode;

#[derive(Educe, GetSize, Clone)]
#[educe(Debug)]
pub struct SlideConfig {
    pub size: CommonSize,
    pub border_width: i32,
    pub obtuse_angle: f64,
    pub radius: f64,
    pub bg_color: Color,
    pub fg_color: Color,
    pub border_color: Color,
    pub fg_text_color: Option<Color>,
    pub bg_text_color: Option<Color>,
    pub redraw_only_on_internal_update: bool,
    pub scroll_unit: f64,
    pub preset: Preset,
}

fn default_scroll_unit() -> f64 {
    0.005
}

fn dt_border_width() -> i32 {
    3
}
fn dt_bg_color() -> Color {
    parse_color("#808080").unwrap()
}
fn dt_fg_color() -> Color {
    parse_color("#FFB847").unwrap()
}
fn dt_border_color() -> Color {
    parse_color("#646464").unwrap()
}
fn dt_obtuse_angle() -> f64 {
    120.
}
fn dt_radius() -> f64 {
    20.
}

impl<S: knus::traits::ErrorSpan> Decode<S> for SlideConfig {
    fn decode_node(
        node: &knus::ast::SpannedNode<S>,
        ctx: &mut knus::decode::Context<S>,
    ) -> Result<Self, knus::errors::DecodeError<S>> {
        let size = CommonSize::decode_node(node, ctx)?;

        let mut border_width = dt_border_width();
        let mut obtuse_angle = dt_obtuse_angle();
        let mut radius = dt_radius();
        let mut bg_color = dt_bg_color();
        let mut fg_color = dt_fg_color();
        let mut border_color = dt_border_color();
        let mut fg_text_color = None;
        let mut bg_text_color = None;
        let mut redraw_only_on_internal_update = false;
        let mut scroll_unit = default_scroll_unit();
        let mut preset = Preset::default();

        for child in node.children() {
            match child.node_name.as_ref() {
                "border-width" => {
                    border_width = argv_v(child, ctx)?;
                }
                "obtuse-angle" => {
                    obtuse_angle = argv_v(child, ctx)?;
                }
                "radius" => {
                    radius = argv_v(child, ctx)?;
                }
                "bg-color" => {
                    bg_color = parse_color(&argv_str(child, ctx)?).to_kdl_error(child)?;
                }
                "fg-color" => {
                    fg_color = parse_color(&argv_str(child, ctx)?).to_kdl_error(child)?;
                }
                "border-color" => {
                    border_color = parse_color(&argv_str(child, ctx)?).to_kdl_error(child)?;
                }
                "fg-text-color" => {
                    fg_text_color = Some(parse_color(&argv_str(child, ctx)?).to_kdl_error(child)?);
                }
                "bg-text-color" => {
                    bg_text_color = Some(parse_color(&argv_str(child, ctx)?).to_kdl_error(child)?);
                }
                "redraw-only-on-internal-update" => {
                    redraw_only_on_internal_update = true;
                }
                "scroll-unit" => {
                    scroll_unit = argv_v(child, ctx)?;
                }
                "preset" => {
                    preset = Preset::decode_node(child, ctx)?;
                }
                _ => {}
            }
        }

        Ok(Self {
            size,
            border_width,
            obtuse_angle,
            radius,
            bg_color,
            fg_color,
            border_color,
            fg_text_color,
            bg_text_color,
            redraw_only_on_internal_update,
            scroll_unit,
            preset,
        })
    }
}
