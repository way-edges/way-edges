use crate::kdl::{
    shared::{CommonSize, KeyEventMap},
    util::{argv_str, argv_v, ToKdlError},
};
use cosmic_text::Color;
use educe::Educe;
use util::color::{parse_color, COLOR_BLACK};
use way_edges_derive::GetSize;

#[derive(Educe, GetSize, Clone)]
#[educe(Debug)]
pub struct BtnConfig {
    pub size: CommonSize,
    pub color: Color,
    pub border_width: i32,
    pub border_color: Color,
    pub event_map: KeyEventMap,
}

impl<S: knus::traits::ErrorSpan> knus::Decode<S> for BtnConfig {
    fn decode_node(
        node: &knus::ast::SpannedNode<S>,
        ctx: &mut knus::decode::Context<S>,
    ) -> Result<Self, knus::errors::DecodeError<S>> {
        let size = CommonSize::decode_node(node, ctx)?;

        let mut color = dt_color();
        let mut border_width = dt_border_width();
        let mut border_color = dt_border_color();
        let mut event_map = KeyEventMap::default();

        for child in node.children() {
            match child.node_name.as_ref() {
                "color" => {
                    color = parse_color(&argv_str(node, ctx)?).to_kdl_error(child)?;
                }
                "border-width" => {
                    border_width = argv_v(child, ctx)?;
                }
                "border-color" => {
                    border_color = parse_color(&argv_str(node, ctx)?).to_kdl_error(child)?;
                }
                "event-map" => {
                    event_map = KeyEventMap::decode_node(child, ctx)?;
                }
                _ => {}
            }
        }

        Ok(Self {
            size,
            color,
            border_width,
            border_color,
            event_map,
        })
    }
}

fn dt_color() -> Color {
    parse_color("#7B98FF").unwrap()
}
fn dt_border_width() -> i32 {
    3
}
fn dt_border_color() -> Color {
    COLOR_BLACK
}
