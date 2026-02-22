use cosmic_text::Color;
use knus::{Decode, DecodeScalar};
use schemars::json_schema;
use schemars::JsonSchema;
use schemars::Schema;
use serde::Deserialize;
use serde::Deserializer;
use serde_json::Value;
use util::color::parse_color;
use way_edges_derive::const_property;
use way_edges_derive::GetSize;

use crate::kdl::shared::{
    color_translate, option_color_translate, schema_color, schema_optional_color, CommonSize, Curve,
};
use crate::kdl::util::{argv, argv_str, argv_v, ToKdlError};

#[derive(Debug, GetSize, Clone, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
#[schemars(transform = WorkspaceConfig_generate_defs)]
#[const_property("type", "workspace")]
#[serde(rename_all = "kebab-case")]
pub struct WorkspaceConfig {
    #[serde(flatten)]
    pub size: CommonSize,
    #[serde(default = "dt_gap")]
    pub gap: i32,
    #[serde(default = "dt_active_increase")]
    pub active_increase: f64,
    #[serde(default = "dt_workspace_transition_duration")]
    pub workspace_transition_duration: u64,
    #[serde(default)]
    pub workspace_animation_curve: Curve,
    #[serde(default = "dt_pop_duration")]
    pub pop_duration: u64,
    #[serde(default = "dt_default_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub default_color: Color,
    #[serde(default = "dt_focus_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub focus_color: Color,
    #[serde(default = "dt_active_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub active_color: Color,
    #[serde(default)]
    #[serde(deserialize_with = "option_color_translate")]
    #[schemars(schema_with = "schema_optional_color")]
    pub hover_color: Option<Color>,
    #[serde(default)]
    pub invert_direction: bool,
    #[serde(default)]
    pub output_name: Option<String>,
    #[serde(default)]
    pub focused_only: bool,
    #[serde(default)]
    pub border_width: Option<i32>,
    #[serde(default = "dt_border_radius")]
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

#[derive(Debug, Clone, JsonSchema)]
#[schemars(transform = WorkspacePreset_generate_defs)]
#[serde(rename_all = "kebab-case")]
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
impl<'de> Deserialize<'de> for WorkspacePreset {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;

        if let Some(preset_str) = value.as_str() {
            match preset_str {
                "hyprland" => Ok(WorkspacePreset::Hyprland),
                "niri" => Ok(WorkspacePreset::Niri(NiriConf::default())),
                _ => Err(serde::de::Error::unknown_variant(
                    preset_str,
                    &["hyprland", "niri"],
                )),
            }
        } else {
            #[derive(Deserialize)]
            #[serde(rename_all = "kebab-case", tag = "type")]
            enum Helper {
                Hyprland,
                Niri(NiriConf),
            }

            let helper: Helper = Helper::deserialize(value).map_err(|err| {
                serde::de::Error::custom(format!("Failed to deserialize as object: {}", err))
            })?;

            match helper {
                Helper::Hyprland => Ok(WorkspacePreset::Hyprland),
                Helper::Niri(conf) => Ok(WorkspacePreset::Niri(conf)),
            }
        }
    }
}

#[derive(Debug, Decode, Clone, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
#[schemars(transform = NiriConf_generate_defs)]
#[const_property("type", "niri")]
#[serde(rename_all = "kebab-case")]
pub struct NiriConf {
    #[knus(child)]
    #[serde(default)]
    pub preserve_empty: bool,
}
impl Default for NiriConf {
    fn default() -> Self {
        Self {
            preserve_empty: false,
        }
    }
}

#[allow(non_snake_case)]
fn WorkspacePreset_generate_defs(s: &mut Schema) {
    *s = json_schema!({
      "oneOf": [
      {
          "type": "string",
          "enum": ["hyprland", "niri"]
      },
      {
        "type": "object",
        "$ref": "#/$defs/NiriConf",
      }
      ]
    })
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
            assert_eq!(
                widget.size.thickness,
                crate::kdl::shared::NumOrRelative::Num(25.0)
            );
            assert_eq!(
                widget.size.length,
                crate::kdl::shared::NumOrRelative::Relative(0.5)
            );
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
