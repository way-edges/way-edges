pub mod conf;
mod raw;

pub use conf::*;
use raw::*;

use crate::ui::EventMap;
use gtk::gdk::RGBA;
use gtk4_layer_shell::{Edge, Layer};
use std::{fs::OpenOptions, io::Read, process::Command, str::FromStr, thread};

fn parse_config(data: &str, group_name: &Option<String>) -> Result<GroupConfig, String> {
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

fn raw_2_conf(raw: RawGroup) -> Result<GroupConfig, String> {
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
            let width = {
                if !raw.width.is_valid_length() {
                    return Err("width must be > 0".to_string());
                }
                raw.width
            };
            let height = {
                if !raw.height.is_valid_length() {
                    return Err(format!("height must be >= 0: {:#?}", raw.height).to_string());
                }
                raw.height
            };
            let event_map = {
                let mut map = EventMap::new();
                for (key, value) in raw.event_map {
                    map.insert(
                        key,
                        Box::new(move || {
                            let value = value.clone();
                            thread::spawn(move || {
                                let mut cmd = Command::new("/bin/sh");
                                let res = cmd.arg("-c").arg(&value).output();
                                if let Err(e) = res {
                                    log::error!("error running command: {value}\nError: {e}");
                                    notify_rust::Notification::new()
                                        .summary("Way-Edges command error")
                                        .body(&format!("Command: {value}\nError: {e}"))
                                        .show()
                                        .unwrap();
                                }
                            });
                        }),
                    );
                }
                map
            };
            let color = match RGBA::from_str(&raw.color) {
                Ok(c) => c,
                Err(e) => {
                    return Err(format!("invalid color {}", e));
                }
            };
            let transition_duration = raw.transition_duration;
            let frame_rate = raw.frame_rate;
            let extra_trigger_size = raw.extra_trigger_size;
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

            Ok(Config {
                edge,
                position,
                layer,
                width,
                height,
                event_map: Some(event_map),
                color,
                transition_duration,
                frame_rate,
                extra_trigger_size,
                monitor,
                margins,
            })
        })
        .collect()
}

fn get_config_file() -> Result<String, String> {
    let bd = match xdg::BaseDirectories::new() {
        Ok(bd) => bd,
        Err(e) => return Err(format!("no xdg base directories: {e}")),
    };

    let p = match bd.place_config_file("way-edges/config.jsonc") {
        Ok(p) => p,
        Err(e) => return Err(format!("failed to create config file: {e}")),
    };

    let mut f = match OpenOptions::new()
        .create(true)
        .append(true)
        .read(true)
        .open(p)
    {
        Ok(f) => f,
        Err(e) => return Err(format!("failed to open config file: {e}")),
    };
    let mut s = String::new();
    match f.read_to_string(&mut s) {
        Ok(_) => {}
        Err(e) => return Err(format!("failed to read config file: {e}")),
    };
    Ok(s)
}

pub fn get_config(group_name: &Option<String>) -> Result<GroupConfig, String> {
    let s = get_config_file()?;
    parse_config(&s, group_name)
}

#[allow(dead_code)]
pub fn get_config_test() {
    let res = get_config(&None).unwrap();
    println!("res: {res:#?}");
}

#[allow(dead_code)]
pub fn parse_config_test() {
    let data = r#"
    {
        "$schema": "sfa",
        "groups": [
            {
                "name": "test",
                "widgets": [{
                    "event_map": [[ 0, "ee" ]]
                }]
            }
        ]
    }
    "#;
    let res = parse_config(data, &None).unwrap();
    println!("{res:#?}");
}
