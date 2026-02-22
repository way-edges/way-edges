use cosmic_text::{Color, FamilyOwned};
use knus::Decode;
use util::color::{parse_color, COLOR_BLACK};

use crate::kdl::{
    shared::{dt_family_owned, parse_family_owned, KeyEventMap},
    util::{argv_str, argv_v},
};

#[derive(Debug, Clone)]
pub enum TextPreset {
    Time {
        format: String,
        time_zone: Option<String>,
        update_interval: u64,
    },
    Custom {
        update_interval: u64,
        cmd: String,
    },
}
impl<S: knus::traits::ErrorSpan> knus::Decode<S> for TextPreset {
    fn decode_node(
        node: &knus::ast::SpannedNode<S>,
        ctx: &mut knus::decode::Context<S>,
    ) -> Result<Self, knus::errors::DecodeError<S>> {
        let mut format = dt_time_format();
        let mut time_zone = None::<String>;
        let mut update_interval = dt_update_interval();
        let mut cmd = String::default();

        match argv_str(node, ctx)?.as_ref() {
            "time" => {
                for child in node.children() {
                    match child.node_name.as_ref() {
                        "format" => {
                            format = argv_str(child, ctx)?;
                        }
                        "time-zone" => {
                            time_zone = Some(argv_str(child, ctx)?);
                        }
                        "update-interval" => {
                            update_interval = argv_v(child, ctx)?;
                        }
                        _ => {}
                    }
                }
                Ok(Self::Time {
                    format,
                    time_zone,
                    update_interval,
                })
            }
            "custom" => {
                for child in node.children() {
                    match child.node_name.as_ref() {
                        "update-interval" => {
                            update_interval = argv_v(child, ctx)?;
                        }
                        "cmd" => {
                            cmd = argv_str(child, ctx)?;
                        }
                        _ => {}
                    }
                }
                Ok(Self::Custom {
                    update_interval,
                    cmd,
                })
            }

            _ => Err(knus::errors::DecodeError::unexpected(
                &node.node_name,
                "\"time\" or \"custom\"",
                "TextPreset node should be \"time\" or \"custom\"",
            )),
        }
    }
}
fn dt_time_format() -> String {
    "%Y-%m-%d %H:%M:%S".to_string()
}
fn dt_update_interval() -> u64 {
    1000
}

#[derive(Debug, Clone, Decode)]
pub struct TextConfig {
    #[knus(child, default = dt_fg_color(), unwrap(argument, decode_with = parse_color))]
    pub fg_color: Color,
    #[knus(child, default = dt_font_size(), unwrap(argument))]
    pub font_size: i32,
    #[knus(child, default = dt_family_owned(), unwrap(argument, decode_with = parse_family_owned))]
    pub font_family: FamilyOwned,
    #[knus(child, default)]
    pub event_map: KeyEventMap,
    #[knus(child)]
    pub preset: TextPreset,
}

fn dt_fg_color() -> Color {
    COLOR_BLACK
}
fn dt_font_size() -> i32 {
    24
}
