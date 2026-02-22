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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_text_configs() {
        let kdl = r##"
wrap-box {
    edge "bottom"
    thickness 20
    length "40%"
    item "text" {
        index 0 0
        preset "time" {
            format "%H:%M"
            time-zone "UTC"
            update-interval 2000
        }
    }
    item "text" {
        index 0 1
        preset "time" {
            format "%Y-%m-%d"
            update-interval 5000
        }
    }
    item "text" {
        index 1 0
        preset "custom" {
            update-interval 3000
            cmd "echo Hello"
        }
        fg-color "#ffffff"
        font-size 30
    }
}
"##;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::WrapBox(wrap_box) = &parsed[0] {
            let config = &wrap_box.widget;
            assert_eq!(config.items.len(), 3);

            // Time preset with format, time-zone, update-interval
            if let crate::kdl::widgets::wrapbox::BoxedWidget::Text(text_config) = &config.items[0].widget {
                assert_eq!(config.items[0].index, [0, 0]);
                match &text_config.preset {
                    TextPreset::Time { format, time_zone, update_interval } => {
                        assert_eq!(format, "%H:%M");
                        assert_eq!(time_zone.as_ref().unwrap(), "UTC");
                        assert_eq!(*update_interval, 2000);
                    }
                    _ => panic!("Expected Time preset"),
                }
                assert_eq!(text_config.fg_color, COLOR_BLACK); // default
                assert_eq!(text_config.font_size, 24); // default
            } else {
                panic!("Expected Text widget");
            }

            // Time preset with format and update-interval, no time-zone
            if let crate::kdl::widgets::wrapbox::BoxedWidget::Text(text_config) = &config.items[1].widget {
                assert_eq!(config.items[1].index, [0, 1]);
                match &text_config.preset {
                    TextPreset::Time { format, time_zone, update_interval } => {
                        assert_eq!(format, "%Y-%m-%d");
                        assert_eq!(time_zone, &None);
                        assert_eq!(*update_interval, 5000);
                    }
                    _ => panic!("Expected Time preset"),
                }
            } else {
                panic!("Expected Text widget");
            }

            // Custom preset with cmd, update-interval, and custom fg-color, font-size
            if let crate::kdl::widgets::wrapbox::BoxedWidget::Text(text_config) = &config.items[2].widget {
                assert_eq!(config.items[2].index, [1, 0]);
                match &text_config.preset {
                    TextPreset::Custom { update_interval, cmd } => {
                        assert_eq!(*update_interval, 3000);
                        assert_eq!(cmd, "echo Hello");
                    }
                    _ => panic!("Expected Custom preset"),
                }
                assert_eq!(text_config.fg_color, parse_color("#ffffff").unwrap());
                assert_eq!(text_config.font_size, 30);
            } else {
                panic!("Expected Text widget");
            }
        } else {
            panic!("Expected WrapBox");
        }
    }
}
