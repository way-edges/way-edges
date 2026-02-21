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

#[cfg(test)]
mod tests {
    use super::*;
    use knus::Decode;

    #[test]
    fn test_decode_minimal_slide_config() {
        let kdl = r##"
slide {
    edge "bottom"
    thickness 20
    length "40%"
}
"##;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::Slide(slide) = &parsed[0] {
            // Assert defaults
            assert_eq!(slide.widget.border_width, dt_border_width());
            assert_eq!(slide.widget.obtuse_angle, dt_obtuse_angle());
            assert_eq!(slide.widget.radius, dt_radius());
            assert_eq!(slide.widget.bg_color, dt_bg_color());
            assert_eq!(slide.widget.fg_color, dt_fg_color());
            assert_eq!(slide.widget.border_color, dt_border_color());
            assert_eq!(slide.widget.fg_text_color, None);
            assert_eq!(slide.widget.bg_text_color, None);
            assert_eq!(slide.widget.redraw_only_on_internal_update, false);
            assert_eq!(slide.widget.scroll_unit, default_scroll_unit());
            assert!(matches!(slide.widget.preset, Preset::Custom(_)));
        } else {
            panic!("Expected Slide");
        }
    }

    #[test]
    fn test_decode_slide_config_with_preset_speaker() {
        let kdl = r##"
slide {
    edge "bottom"
    thickness 20
    length "40%"
    preset "speaker" {
        mute-color "#ff0000"
        device "alsa_output.pci-0000_00_1b.0.analog-stereo"
    }
}
"##;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::Slide(slide) = &parsed[0] {
            assert!(matches!(slide.widget.preset, Preset::Speaker(_)));
            if let Preset::Speaker(conf) = &slide.widget.preset {
                assert_eq!(conf.mute_color, parse_color("#ff0000").unwrap());
                assert_eq!(
                    conf.device,
                    Some("alsa_output.pci-0000_00_1b.0.analog-stereo".to_string())
                );
            }
        } else {
            panic!("Expected Slide");
        }
    }

    #[test]
    fn test_decode_slide_config_with_preset_backlight() {
        let kdl = r##"
slide {
    edge "bottom"
    thickness 20
    length "40%"
    preset "backlight" {
        device "intel_backlight"
    }
}
"##;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::Slide(slide) = &parsed[0] {
            assert!(matches!(slide.widget.preset, Preset::Backlight(_)));
            if let Preset::Backlight(conf) = &slide.widget.preset {
                assert_eq!(conf.device, Some("intel_backlight".to_string()));
            }
        } else {
            panic!("Expected Slide");
        }
    }

    #[test]
    fn test_decode_slide_config_with_preset_custom() {
        let kdl = r##"
slide {
    edge "bottom"
    thickness 20
    length "40%"
    preset "custom" {
        update-command "echo test"
        update-interval 1000
        on-change-command "notify-send {}"
        event-map {}
    }
}
"##;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::Slide(slide) = &parsed[0] {
            assert!(matches!(slide.widget.preset, Preset::Custom(_)));
            if let Preset::Custom(conf) = &slide.widget.preset {
                assert_eq!(conf.update_command, "echo test");
                assert_eq!(conf.update_interval, 1000);
                assert!(conf.on_change_command.is_some());
            }
        } else {
            panic!("Expected Slide");
        }
    }

    #[test]
    fn test_decode_slide_config_invalid_bg_color() {
        let kdl = r#"
slide {
    edge "bottom"
    thickness 20
    length "40%"
    bg-color "invalid-color"
}
"#;
        let result: Result<Vec<crate::kdl::TopLevelConf>, _> = knus::parse("test", kdl);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_slide_config_invalid_border_width() {
        let kdl = r#"
slide {
    edge "bottom"
    thickness 20
    length "40%"
    border-width "not-a-number"
}
"#;
        let result: Result<Vec<crate::kdl::TopLevelConf>, _> = knus::parse("test", kdl);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_slide_config_invalid_preset() {
        let kdl = r#"
slide {
    edge "bottom"
    thickness 20
    length "40%"
    preset "invalid-preset"
}
"#;
        let result: Result<Vec<crate::kdl::TopLevelConf>, _> = knus::parse("test", kdl);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_slide_config_border_width() {
        let kdl = r#"
slide {
    edge "bottom"
    thickness 20
    length "40%"
    border-width 5
}
"#;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::Slide(slide) = &parsed[0] {
            assert_eq!(slide.widget.border_width, 5);
        } else {
            panic!("Expected Slide");
        }
    }

    #[test]
    fn test_decode_slide_config_obtuse_angle() {
        let kdl = r#"
slide {
    edge "bottom"
    thickness 20
    length "40%"
    obtuse-angle 150.0
}
"#;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::Slide(slide) = &parsed[0] {
            assert_eq!(slide.widget.obtuse_angle, 150.0);
        } else {
            panic!("Expected Slide");
        }
    }

    #[test]
    fn test_decode_slide_config_radius() {
        let kdl = r#"
slide {
    edge "bottom"
    thickness 20
    length "40%"
    radius 25.0
}
"#;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::Slide(slide) = &parsed[0] {
            assert_eq!(slide.widget.radius, 25.0);
        } else {
            panic!("Expected Slide");
        }
    }

    #[test]
    fn test_decode_slide_config_colors() {
        let kdl = r##"
slide {
    edge "bottom"
    thickness 20
    length "40%"
    bg-color "#ffffff"
    fg-color "#000000"
    border-color "#cccccc"
    fg-text-color "#ff0000"
    bg-text-color "#00ff00"
}
"##;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::Slide(slide) = &parsed[0] {
            assert_eq!(slide.widget.bg_color, parse_color("#ffffff").unwrap());
            assert_eq!(slide.widget.fg_color, parse_color("#000000").unwrap());
            assert_eq!(slide.widget.border_color, parse_color("#cccccc").unwrap());
            assert_eq!(
                slide.widget.fg_text_color,
                Some(parse_color("#ff0000").unwrap())
            );
            assert_eq!(
                slide.widget.bg_text_color,
                Some(parse_color("#00ff00").unwrap())
            );
        } else {
            panic!("Expected Slide");
        }
    }

    #[test]
    fn test_decode_slide_config_scroll_unit() {
        let kdl = r#"
slide {
    edge "bottom"
    thickness 20
    length "40%"
    scroll-unit 0.01
}
"#;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::Slide(slide) = &parsed[0] {
            assert_eq!(slide.widget.scroll_unit, 0.01);
        } else {
            panic!("Expected Slide");
        }
    }

    #[test]
    fn test_decode_slide_config_redraw_only_on_internal_update() {
        let kdl = r#"
slide {
    edge "bottom"
    thickness 20
    length "40%"
    redraw-only-on-internal-update
}
"#;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::Slide(slide) = &parsed[0] {
            assert_eq!(slide.widget.redraw_only_on_internal_update, true);
        } else {
            panic!("Expected Slide");
        }
    }

    #[test]
    fn test_decode_slide_config_all_fields() {
        let kdl = r##"
slide {
    edge "top"
    thickness 25
    length "50%"
    border-width 5
    obtuse-angle 140.0
    radius 30.0
    bg-color "#aaaaaa"
    fg-color "#bbbbbb"
    border-color "#cccccc"
    fg-text-color "#dddddd"
    bg-text-color "#eeeeee"
    redraw-only-on-internal-update
    scroll-unit 0.02
    preset "custom" {
        update-command "test command"
        update-interval 2000
        on-change-command "notify {}"
            event-map {}
    }
}
"##;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::Slide(slide) = &parsed[0] {
            let widget = &slide.widget;
            assert_eq!(widget.border_width, 5);
            assert_eq!(widget.obtuse_angle, 140.0);
            assert_eq!(widget.radius, 30.0);
            assert_eq!(widget.bg_color, parse_color("#aaaaaa").unwrap());
            assert_eq!(widget.fg_color, parse_color("#bbbbbb").unwrap());
            assert_eq!(widget.border_color, parse_color("#cccccc").unwrap());
            assert_eq!(widget.fg_text_color, Some(parse_color("#dddddd").unwrap()));
            assert_eq!(widget.bg_text_color, Some(parse_color("#eeeeee").unwrap()));
            assert_eq!(widget.redraw_only_on_internal_update, true);
            assert_eq!(widget.scroll_unit, 0.02);
            assert!(matches!(widget.preset, Preset::Custom(_)));
            if let Preset::Custom(conf) = &widget.preset {
                assert_eq!(conf.update_command, "test command");
                assert_eq!(conf.update_interval, 2000);
                assert!(conf.on_change_command.is_some());
            }
        } else {
            panic!("Expected Slide");
        }
    }
}
