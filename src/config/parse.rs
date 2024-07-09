use crate::activate::MonitorSpecifier;

use super::conf::*;
use super::raw::*;
use super::widgets;

use gtk4_layer_shell::{Edge, Layer};
use serde_jsonrc::Value;

pub fn parse_config(data: &str, group_name: &Option<String>) -> Result<GroupConfig, String> {
    let mut res: RawTemp =
        serde_jsonrc::from_str(data).map_err(|e| format!("JSON parse error: {e}"))?;
    let group = if let Some(s) = group_name {
        res.groups
            .into_iter()
            .find(|g| &g.name == s)
            .ok_or_else(|| format!("group {s} not found"))?
    } else {
        res.groups.remove(0)
    };
    raw_2_conf(group)
}

pub fn raw_2_conf(raw: RawGroup) -> Result<GroupConfig, String> {
    raw.widgets
        .into_iter()
        .map(|raw| -> Result<Config, String> {
            let edge = match raw.edge.as_str() {
                "top" => Edge::Top,
                "left" => Edge::Left,
                "bottom" => Edge::Bottom,
                "right" => Edge::Right,
                _ => {
                    return Err(format!("invalid edge {}", raw.edge));
                }
            };
            let position = match raw.position.as_str() {
                "top" => Some(Edge::Top),
                "left" => Some(Edge::Left),
                "bottom" => Some(Edge::Bottom),
                "right" => Some(Edge::Right),
                "" | "center" => None,
                _ => {
                    return Err(format!("invalid position {}", raw.position));
                }
            };
            let layer = match raw.layer.as_str() {
                "background" => Layer::Background,
                "bottom" => Layer::Bottom,
                "top" => Layer::Top,
                "overlay" => Layer::Overlay,
                _ => {
                    return Err(format!("invalid layer {}", raw.layer));
                }
            };
            let monitor = {
                if raw.monitor_name.is_empty() {
                    MonitorSpecifier::ID(raw.monitor_id)
                } else {
                    MonitorSpecifier::Name(raw.monitor_name)
                }
            };
            let margins = {
                let mut m = Vec::new();
                if raw.margin.left.is_valid_length() {
                    m.push((Edge::Left, raw.margin.left))
                }
                if raw.margin.right.is_valid_length() {
                    m.push((Edge::Right, raw.margin.right))
                }
                if raw.margin.top.is_valid_length() {
                    m.push((Edge::Top, raw.margin.top))
                }
                if raw.margin.bottom.is_valid_length() {
                    m.push((Edge::Bottom, raw.margin.bottom))
                }
                m
            };
            let widget = parse_widget(raw.widget)?;

            Ok(Config {
                edge,
                position,
                layer,
                monitor,
                margins,
                widget: Some(widget),
            })
        })
        .collect()
}

fn parse_widget(raw: Value) -> Result<Widget, String> {
    if !raw.is_object() {
        return Err("Widget must be object".to_string());
    }
    let t = raw
        .get("type")
        .ok_or("widget must have type")?
        .as_str()
        .ok_or("widget type must be string")?;
    let w = match t {
        "btn" => widgets::button::visit_btn_config(raw).map_err(|e| e.to_string())?,
        "slide" => Widget::Slider,
        _ => return Err(format!("unknown widget type: {t}")),
    };
    Ok(w)
}
