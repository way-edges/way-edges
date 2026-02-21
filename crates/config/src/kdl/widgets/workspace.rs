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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_workspace_config_with_preset() {
        let kdl = r#"
workspace {
    edge "bottom"
    thickness 20
    length "40%"
    preset "niri" {
        preserve-empty
    }
}
"#;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::Workspace(ws) = &parsed[0] {
            assert!(matches!(ws.widget.preset, WorkspacePreset::Niri(_)));
            if let WorkspacePreset::Niri(conf) = &ws.widget.preset {
                assert!(conf.preserve_empty);
            }
        } else {
            panic!("Expected Workspace");
        }
    }

    #[test]
    fn test_decode_workspace_config_invalid_default_color() {
        let kdl = r#"
workspace {
    edge "bottom"
    thickness 20
    length "40%"
    default-color "invalid-color"
}
"#;
        let result: Result<Vec<crate::kdl::TopLevelConf>, _> = knus::parse("test", kdl);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_workspace_config_invalid_preset() {
        let kdl = r#"
workspace {
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
    fn test_decode_workspace_config_invalid_border_width() {
        let kdl = r#"
workspace {
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
    fn test_decode_workspace_config_defaults() {
        let kdl = r#"
workspace {
    edge "bottom"
    thickness 20
    length "40%"
}
"#;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::Workspace(ws) = &parsed[0] {
            let widget = &ws.widget;
            assert_eq!(widget.gap, 5);
            assert_eq!(widget.active_increase, 0.5);
            assert_eq!(widget.workspace_transition_duration, 300);
            assert_eq!(widget.workspace_animation_curve, Curve::EaseCubic);
            assert_eq!(widget.pop_duration, 1000);
            assert_eq!(widget.default_color, parse_color("#003049").unwrap());
            assert_eq!(widget.focus_color, parse_color("#669bbc").unwrap());
            assert_eq!(widget.active_color, parse_color("#aaa").unwrap());
            assert_eq!(widget.hover_color, None);
            assert_eq!(widget.invert_direction, false);
            assert_eq!(widget.output_name, None);
            assert_eq!(widget.focused_only, false);
            assert_eq!(widget.border_width, None);
            assert_eq!(widget.border_radius, 5);
            assert!(matches!(widget.preset, WorkspacePreset::Hyprland));
        } else {
            panic!("Expected Workspace");
        }
    }

    #[test]
    fn test_decode_workspace_config_gap() {
        let kdl = r#"
workspace {
    edge "bottom"
    thickness 20
    length "40%"
    gap 10
}
"#;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::Workspace(ws) = &parsed[0] {
            assert_eq!(ws.widget.gap, 10);
        } else {
            panic!("Expected Workspace");
        }
    }

    #[test]
    fn test_decode_workspace_config_active_increase() {
        let kdl = r#"
workspace {
    edge "bottom"
    thickness 20
    length "40%"
    active-increase 0.7
}
"#;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::Workspace(ws) = &parsed[0] {
            assert_eq!(ws.widget.active_increase, 0.7);
        } else {
            panic!("Expected Workspace");
        }
    }

    #[test]
    fn test_decode_workspace_config_workspace_transition_duration() {
        let kdl = r#"
workspace {
    edge "bottom"
    thickness 20
    length "40%"
    workspace-transition-duration 500
}
"#;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::Workspace(ws) = &parsed[0] {
            assert_eq!(ws.widget.workspace_transition_duration, 500);
        } else {
            panic!("Expected Workspace");
        }
    }

    #[test]
    fn test_decode_workspace_config_workspace_animation_curve() {
        let kdl = r#"
workspace {
    edge "bottom"
    thickness 20
    length "40%"
    workspace-animation-curve "ease-quad"
}
"#;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::Workspace(ws) = &parsed[0] {
            assert_eq!(ws.widget.workspace_animation_curve, Curve::EaseQuad);
        } else {
            panic!("Expected Workspace");
        }
    }

    #[test]
    fn test_decode_workspace_config_pop_duration() {
        let kdl = r#"
workspace {
    edge "bottom"
    thickness 20
    length "40%"
    pop-duration 1500
}
"#;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::Workspace(ws) = &parsed[0] {
            assert_eq!(ws.widget.pop_duration, 1500);
        } else {
            panic!("Expected Workspace");
        }
    }

    #[test]
    fn test_decode_workspace_config_all_fields() {
        let kdl = r##"
workspace {
    edge "top"
    thickness 25
    length "50%"
    gap 15
    active-increase 0.8
    workspace-transition-duration 600
    workspace-animation-curve "ease-quad"
    pop-duration 1200
    default-color "#ff0000"
    focus-color "#00ff00"
    active-color "#0000ff"
    hover-color "#ffff00"
    invert-direction
    output-name "DP-1"
    focused-only
    border-width 3
    border-radius 12
    preset "niri" {
        preserve-empty
    }
}
"##;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::Workspace(ws) = &parsed[0] {
            let widget = &ws.widget;
            assert_eq!(widget.size.thickness, crate::kdl::shared::NumOrRelative::Num(25.0));
            assert_eq!(widget.size.length, crate::kdl::shared::NumOrRelative::Relative(0.5));
            assert_eq!(widget.gap, 15);
            assert_eq!(widget.active_increase, 0.8);
            assert_eq!(widget.workspace_transition_duration, 600);
            assert_eq!(widget.workspace_animation_curve, Curve::EaseQuad);
            assert_eq!(widget.pop_duration, 1200);
            assert_eq!(widget.default_color, parse_color("#ff0000").unwrap());
            assert_eq!(widget.focus_color, parse_color("#00ff00").unwrap());
            assert_eq!(widget.active_color, parse_color("#0000ff").unwrap());
            assert_eq!(widget.hover_color, Some(parse_color("#ffff00").unwrap()));
            assert_eq!(widget.invert_direction, true);
            assert_eq!(widget.output_name, Some("DP-1".to_string()));
            assert_eq!(widget.focused_only, true);
            assert_eq!(widget.border_width, Some(3));
            assert_eq!(widget.border_radius, 12);
            assert!(matches!(widget.preset, WorkspacePreset::Niri(_)));
            if let WorkspacePreset::Niri(conf) = &widget.preset {
                assert!(conf.preserve_empty);
            }
        } else {
            panic!("Expected Workspace");
        }
    }
}
