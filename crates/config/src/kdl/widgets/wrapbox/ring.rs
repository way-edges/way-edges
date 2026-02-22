use cosmic_text::{Color, FamilyOwned};
use knus::Decode;
use util::color::parse_color;
use util::template::{
    arg::{TemplateArgFloatProcesser, TemplateArgRingPresetProcesser},
    base::{Template, TemplateProcesser},
};

use crate::kdl::shared::{dt_family_owned, parse_family_owned, Curve, KeyEventMap};
use crate::kdl::util::{argv_str, argv_v};

#[derive(Debug, Clone)]
pub enum RingPreset {
    Ram {
        update_interval: u64,
    },
    Swap {
        update_interval: u64,
    },
    Cpu {
        update_interval: u64,
        core: Option<usize>,
    },
    Battery {
        update_interval: u64,
    },
    Disk {
        update_interval: u64,
        partition: String,
    },
    Custom {
        update_interval: u64,
        cmd: String,
    },
}
impl Default for RingPreset {
    fn default() -> Self {
        Self::Custom {
            update_interval: dt_update_interval(),
            cmd: String::default(),
        }
    }
}
fn dt_partition() -> String {
    "/".to_string()
}
fn dt_update_interval() -> u64 {
    1000
}

impl<S: knus::traits::ErrorSpan> knus::Decode<S> for RingPreset {
    fn decode_node(
        node: &knus::ast::SpannedNode<S>,
        ctx: &mut knus::decode::Context<S>,
    ) -> Result<Self, knus::errors::DecodeError<S>> {
        let mut update_interval = dt_update_interval();
        let mut partition = dt_partition();
        let mut cmd = String::default();

        match argv_str(node, ctx)?.as_ref() {
            "ram" => {
                for child in node.children() {
                    match child.node_name.as_ref() {
                        "update-interval" => {
                            update_interval = argv_v(child, ctx)?;
                        }
                        _ => {}
                    }
                }
                Ok(Self::Ram { update_interval })
            }
            "swap" => {
                for child in node.children() {
                    match child.node_name.as_ref() {
                        "update-interval" => {
                            update_interval = argv_v(child, ctx)?;
                        }
                        _ => {}
                    }
                }
                Ok(Self::Swap { update_interval })
            }
            "cpu" => {
                let mut core = None;
                for child in node.children() {
                    match child.node_name.as_ref() {
                        "update-interval" => {
                            update_interval = argv_v(child, ctx)?;
                        }
                        "core" => {
                            core = Some(argv_v(child, ctx)?);
                        }
                        _ => {}
                    }
                }
                Ok(Self::Cpu { update_interval, core })
            }
            "battery" => {
                for child in node.children() {
                    match child.node_name.as_ref() {
                        "update-interval" => {
                            update_interval = argv_v(child, ctx)?;
                        }
                        _ => {}
                    }
                }
                Ok(Self::Battery { update_interval })
            }
            "disk" => {
                for child in node.children() {
                    match child.node_name.as_ref() {
                        "update-interval" => {
                            update_interval = argv_v(child, ctx)?;
                        }
                        "partition" => {
                            partition = argv_str(child, ctx)?;
                        }
                        _ => {}
                    }
                }
                Ok(Self::Disk { update_interval, partition })
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
                Ok(Self::Custom { update_interval, cmd })
            }

            _ => Err(knus::errors::DecodeError::unexpected(
                &node.node_name,
                "\"ram\", \"swap\", \"cpu\", \"battery\", \"disk\" or \"custom\"",
                "RingPreset node should be one of \"ram\", \"swap\", \"cpu\", \"battery\", \"disk\" or \"custom\"",
            )),
        }
    }
}

#[derive(Debug, Decode, Clone)]
pub struct RingConfig {
    #[knus(child, default = dt_r(), unwrap(argument))]
    pub radius: i32,
    #[knus(child, default = dt_rw(), unwrap(argument))]
    pub ring_width: i32,
    #[knus(child, default = dt_bg(), unwrap(argument, decode_with = parse_color))]
    pub bg_color: Color,
    #[knus(child, default = dt_fg(), unwrap(argument, decode_with = parse_color))]
    pub fg_color: Color,
    #[knus(child, default = dt_tt(), unwrap(argument))]
    pub text_transition_ms: u64,
    #[knus(child, default, unwrap(argument))]
    pub animation_curve: Curve,
    #[knus(child, default, unwrap(argument, decode_with = ring_text_optional_template))]
    pub prefix: Option<Template>,
    #[knus(child)]
    pub prefix_hide: bool,
    #[knus(child, default, unwrap(argument, decode_with = ring_text_optional_template))]
    pub suffix: Option<Template>,
    #[knus(child)]
    pub suffix_hide: bool,
    #[knus(child, default = dt_family_owned(), unwrap(argument, decode_with = parse_family_owned))]
    pub font_family: FamilyOwned,
    #[knus(child, default, unwrap(argument))]
    // let font_size = value.font_size.unwrap_or(value.radius * 2);
    pub font_size: Option<i32>,
    #[knus(child, default)]
    pub event_map: KeyEventMap,
    #[knus(child)]
    pub preset: RingPreset,
}

fn dt_r() -> i32 {
    13
}
fn dt_rw() -> i32 {
    5
}
fn dt_bg() -> Color {
    parse_color("#9F9F9F").unwrap()
}
fn dt_fg() -> Color {
    parse_color("#F1FA8C").unwrap()
}
fn dt_tt() -> u64 {
    300
}

fn ring_text_optional_template(s: &str) -> Result<Option<Template>, String> {
    if s.is_empty() {
        Ok(None)
    } else {
        Template::create_from_str(
            s,
            TemplateProcesser::new()
                .add_processer(TemplateArgFloatProcesser)
                .add_processer(TemplateArgRingPresetProcesser),
        )
        .map(Some)
        .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_ring_configs() {
        let kdl = r##"
wrap-box {
    edge "bottom"
    thickness 20
    length "40%"
    item "ring" {
        index 0 0
        preset "ram" {
            update-interval 2000
        }
    }
    item "ring" {
        index 0 1
        preset "cpu" {
            update-interval 1500
            core 1
        }
    }
    item "ring" {
        index 1 0
        preset "disk" {
            update-interval 3000
            partition "/home"
        }
    }
    item "ring" {
        index 1 1
        preset "custom" {
            update-interval 5000
            cmd "echo 50"
        }
        radius 20
        ring-width 8
        bg-color "#000000"
        fg-color "#ff0000"
        text-transition-ms 500
        prefix "Usage: "
        suffix "%"
    }
}
"##;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::WrapBox(wrap_box) = &parsed[0] {
            let config = &wrap_box.widget;
            assert_eq!(config.items.len(), 4);

            // Ram preset
            if let crate::kdl::widgets::wrapbox::BoxedWidget::Ring(ring_config) = &config.items[0].widget {
                assert_eq!(config.items[0].index, [0, 0]);
                match &ring_config.preset {
                    RingPreset::Ram { update_interval } => {
                        assert_eq!(*update_interval, 2000);
                    }
                    _ => panic!("Expected Ram preset"),
                }
                assert_eq!(ring_config.radius, 13); // default
                assert_eq!(ring_config.ring_width, 5); // default
            } else {
                panic!("Expected Ring widget");
            }

            // Cpu preset with core
            if let crate::kdl::widgets::wrapbox::BoxedWidget::Ring(ring_config) = &config.items[1].widget {
                assert_eq!(config.items[1].index, [0, 1]);
                match &ring_config.preset {
                    RingPreset::Cpu { update_interval, core } => {
                        assert_eq!(*update_interval, 1500);
                        assert_eq!(*core, Some(1));
                    }
                    _ => panic!("Expected Cpu preset"),
                }
            } else {
                panic!("Expected Ring widget");
            }

            // Disk preset with partition
            if let crate::kdl::widgets::wrapbox::BoxedWidget::Ring(ring_config) = &config.items[2].widget {
                assert_eq!(config.items[2].index, [1, 0]);
                match &ring_config.preset {
                    RingPreset::Disk { update_interval, partition } => {
                        assert_eq!(*update_interval, 3000);
                        assert_eq!(partition, "/home");
                    }
                    _ => panic!("Expected Disk preset"),
                }
            } else {
                panic!("Expected Ring widget");
            }

            // Custom preset with cmd and other fields
            if let crate::kdl::widgets::wrapbox::BoxedWidget::Ring(ring_config) = &config.items[3].widget {
                assert_eq!(config.items[3].index, [1, 1]);
                match &ring_config.preset {
                    RingPreset::Custom { update_interval, cmd } => {
                        assert_eq!(*update_interval, 5000);
                        assert_eq!(cmd, "echo 50");
                    }
                    _ => panic!("Expected Custom preset"),
                }
                assert_eq!(ring_config.radius, 20);
                assert_eq!(ring_config.ring_width, 8);
                assert_eq!(ring_config.bg_color, parse_color("#000000").unwrap());
                assert_eq!(ring_config.fg_color, parse_color("#ff0000").unwrap());
                assert_eq!(ring_config.text_transition_ms, 500);
                assert!(ring_config.prefix.is_some());
                assert!(ring_config.suffix.is_some());
            } else {
                panic!("Expected Ring widget");
            }
        } else {
            panic!("Expected WrapBox");
        }
    }
}
