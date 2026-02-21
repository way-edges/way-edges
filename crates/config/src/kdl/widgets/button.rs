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

#[cfg(test)]
mod tests {
    use super::*;
    use knus::Decode;

    #[test]
    fn test_decode_minimal_btn_config() {
        let kdl = r#"
btn {
    edge "bottom"
    thickness 20
    length "40%"
}
"#;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::Btn(btn) = &parsed[0] {
            // Assert defaults
            assert_eq!(btn.widget.color, dt_color());
            assert_eq!(btn.widget.border_width, dt_border_width());
            assert_eq!(btn.widget.border_color, dt_border_color());
            assert!(btn.widget.event_map.is_empty());
        } else {
            panic!("Expected Btn");
        }
    }

    #[test]
    fn test_decode_btn_config_with_event_map() {
        let kdl = r#"
btn {
    edge "bottom"
    thickness 20
    length "40%"
    event-map {
        mouse-left "some command"
        mouse-right "another command"
    }
}
"#;
        let parsed: Vec<crate::kdl::TopLevelConf> = knus::parse("test", kdl).unwrap();
        if let crate::kdl::TopLevelConf::Btn(btn) = &parsed[0] {
            assert_eq!(btn.widget.color, dt_color());
            assert_eq!(btn.widget.border_width, dt_border_width());
            assert_eq!(btn.widget.border_color, dt_border_color());
            assert_eq!(btn.widget.event_map.len(), 2);
            assert_eq!(
                btn.widget.event_map.get(&0x110),
                Some(&"some command".to_string())
            );
            assert_eq!(
                btn.widget.event_map.get(&0x111),
                Some(&"another command".to_string())
            );
        } else {
            panic!("Expected Btn");
        }
    }

    #[test]
    fn test_decode_btn_config_invalid_color() {
        let kdl = r#"
btn {
    thickness 20
    length "40%"
    color "invalid-color"
}
"#;
        let result: Result<Vec<crate::kdl::TopLevelConf>, _> = knus::parse("test", kdl);
        assert!(result.is_err()); // Should fail due to invalid color
    }

    #[test]
    fn test_decode_btn_config_invalid_border_width() {
        let kdl = r#"
btn {
    thickness 20
    length "40%"
    border-width "not-a-number"
}
"#;
        let result: Result<Vec<crate::kdl::TopLevelConf>, _> = knus::parse("test", kdl);
        assert!(result.is_err()); // Should fail due to invalid border-width
    }

    #[test]
    fn test_decode_btn_config_invalid_border_color() {
        let kdl = r#"
btn {
    thickness 20
    length "40%"
    border-color "invalid-color"
}
"#;
        let result: Result<Vec<crate::kdl::TopLevelConf>, _> = knus::parse("test", kdl);
        assert!(result.is_err()); // Should fail due to invalid border-color
    }

    #[test]
    fn test_decode_btn_config_invalid_event_map() {
        let kdl = r#"
btn {
    thickness 20
    length "40%"
    event-map {
        invalid-key "command"
    }
}
"#;
        let result: Result<Vec<crate::kdl::TopLevelConf>, _> = knus::parse("test", kdl);
        assert!(result.is_err()); // Should fail due to invalid event key
    }
}
