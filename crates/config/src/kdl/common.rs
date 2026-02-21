use knus::{
    errors::DecodeError,
    traits::{DecodePartial, ErrorSpan},
    Decode,
};
use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer};
use std::collections::HashSet;
use std::ops::Deref;

use crate::kdl::shared::{Curve, NumOrRelative};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MonitorSpecifier {
    Lists {
        ids: HashSet<usize>,
        namses: HashSet<String>,
    },
    All,
}
impl Default for MonitorSpecifier {
    fn default() -> Self {
        Self::Lists {
            ids: HashSet::from([0]),
            namses: HashSet::new(),
        }
    }
}
impl<S: knus::traits::ErrorSpan> knus::Decode<S> for MonitorSpecifier {
    fn decode_node(
        node: &knus::ast::SpannedNode<S>,
        ctx: &mut knus::decode::Context<S>,
    ) -> Result<Self, DecodeError<S>> {
        // check empty
        if node.arguments.is_empty() {
            return Err(DecodeError::unexpected(
                &node.node_name,
                "index or name",
                "MonitorSpecifier should have at least one argument",
            ));
        }
        // knus::Decode::D

        #[allow(clippy::collapsible_if)]
        if node.arguments.len() == 1 {
            if let knus::ast::Literal::String(s) = node.arguments[0].literal.deref() {
                if s.deref() == "*" {
                    return Ok(MonitorSpecifier::All);
                }
            }
        }

        let mut ids = HashSet::new();
        let mut names = HashSet::new();

        for arg in &node.arguments {
            match arg.literal.deref() {
                knus::ast::Literal::String(s) => {
                    if s.deref() == "*" {
                        return Err(DecodeError::unsupported(
                            &arg.literal,
                            "You cannot use the wildcard character '*' in a list of monitors, it is only allowed as the sole argument to specify all monitors",
                        ));
                    }
                    names.insert(s.to_string());
                }
                knus::ast::Literal::Int(value) => {
                    if let Ok(id) = value.try_into() {
                        ids.insert(id);
                    } else {
                        return Err(DecodeError::unsupported(
                            &arg.literal,
                            "Invalid integer value encountered",
                        ));
                    }
                }
                _ => {
                    return Err(DecodeError::unsupported(
                        &arg.literal,
                        "Unsupported value, only numbers and strings are recognized",
                    ));
                }
            }
        }

        Ok(MonitorSpecifier::Lists { ids, namses: names })
    }
}

#[derive(Debug, Clone, Default, Decode)]
pub struct Margins {
    #[knus(child, default, unwrap(argument))]
    pub left: NumOrRelative,
    #[knus(child, default, unwrap(argument))]
    pub top: NumOrRelative,
    #[knus(child, default, unwrap(argument))]
    pub right: NumOrRelative,
    #[knus(child, default, unwrap(argument))]
    pub bottom: NumOrRelative,
}

#[derive(Debug, Clone, Decode)]
pub struct CommonConfig {
    #[knus(child, unwrap(argument, decode_with = match_edge))]
    pub edge: Anchor,

    #[knus(child, unwrap(argument, decode_with = match_edge))]
    pub position: Option<Anchor>,

    #[knus(child, default=Layer::Top, unwrap(argument, decode_with = match_layer))]
    pub layer: Layer,

    #[knus(child, default, unwrap(argument))]
    pub offset: NumOrRelative,

    #[knus(child, default)]
    pub margins: Margins,

    #[knus(child, default)]
    pub monitor: MonitorSpecifier,

    #[knus(child, default, unwrap(argument))]
    pub namespace: String,

    #[knus(child)]
    pub ignore_exclusive: bool,

    #[knus(child, default = 300, unwrap(argument))]
    pub transition_duration: u64,

    #[knus(child, default, unwrap(argument))]
    pub animation_curve: Curve,

    #[knus(child, default = NumOrRelative::Num(1.0), unwrap(argument))]
    pub extra_trigger_size: NumOrRelative,

    #[knus(child, default = NumOrRelative::Num(0.0), unwrap(argument))]
    pub preview_size: NumOrRelative,

    // TODO: true
    #[knus(child)]
    pub pinnable: bool,

    // TODO: true
    #[knus(child)]
    pub pin_with_key: bool,

    #[knus(child, default = smithay_client_toolkit::seat::pointer::BTN_MIDDLE, unwrap(argument))]
    pub pin_key: u32,

    #[knus(child)]
    pub pin_on_startup: bool,
}

impl CommonConfig {
    pub fn resolve_relative(&mut self, size: (i32, i32)) {
        // margins
        macro_rules! calculate_margins {
            ($m:expr, $s:expr) => {
                if $m.is_relative() {
                    $m.calculate_relative($s as f64);
                }
            };
        }
        calculate_margins!(self.margins.left, size.0);
        calculate_margins!(self.margins.right, size.0);
        calculate_margins!(self.margins.top, size.1);
        calculate_margins!(self.margins.bottom, size.1);

        // offset & extra
        let max = match self.edge {
            Anchor::LEFT | Anchor::RIGHT => size.0,
            Anchor::TOP | Anchor::BOTTOM => size.1,
            _ => unreachable!(),
        };
        if self.offset.is_relative() {
            self.offset.calculate_relative(max as f64);
        }
        if self.extra_trigger_size.is_relative() {
            self.extra_trigger_size.calculate_relative(max as f64);
        }
    }
}

fn match_edge(edge: &str) -> Result<Anchor, std::io::Error> {
    Ok(match edge {
        "top" => Anchor::TOP,
        "left" => Anchor::LEFT,
        "bottom" => Anchor::BOTTOM,
        "right" => Anchor::RIGHT,
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid edge: {}", edge),
            ))
        }
    })
}

fn match_layer(layer: &str) -> Result<Layer, std::io::Error> {
    Ok(match layer {
        "background" => Layer::Background,
        "bottom" => Layer::Bottom,
        "top" => Layer::Top,
        "overlay" => Layer::Overlay,
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid layer: {}", layer),
            ))
        }
    })
}
