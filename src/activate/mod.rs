pub mod default;
pub mod hyprland;

use crate::config::{Config, GroupConfig, MonitorSpecifier, NumOrRelative};
use crate::ui;
use gio::prelude::*;
use gtk::gdk::Monitor;
use gtk::prelude::{DisplayExt, MonitorExt};
use gtk::Application;
use gtk4_layer_shell::{Edge, LayerShell};

fn get_monitors() -> gio::ListModel {
    let dt_display = gtk::gdk::Display::default().expect("display for monitor not found");
    dt_display.monitors()
}

fn find_monitor(monitors: &gio::ListModel, specifier: MonitorSpecifier) -> Monitor {
    let a = match specifier {
        MonitorSpecifier::ID(index) => {
            let a = monitors
                .iter::<Monitor>()
                .nth(index)
                .unwrap_or_else(|| panic!("error matching monitor with id: {index}"))
                .unwrap_or_else(|_| panic!("error matching monitor with id: {index}"));
            a
        }
        MonitorSpecifier::Name(name) => {
            for m in monitors.iter() {
                let m: Monitor = m.unwrap();
                if m.connector().unwrap() == name {
                    return m;
                }
            }
            panic!("monitor with name: {name} not found");
        }
    };
    a
}

fn calculate_relative(cfg: &mut Config, max_size_raw: (i32, i32)) {
    let max_size = match cfg.edge {
        Edge::Left | Edge::Right => (max_size_raw.0, max_size_raw.1),
        Edge::Top | Edge::Bottom => (max_size_raw.1, max_size_raw.0),
        _ => unreachable!(),
    };
    println!("max_size: {max_size:?}");
    if let Ok(r) = cfg.width.get_rel() {
        cfg.width = NumOrRelative::Num(max_size.0 as f64 * r);
    };
    if let Ok(r) = cfg.height.get_rel() {
        cfg.height = NumOrRelative::Num(max_size.1 as f64 * r);
    };
    if let Ok(r) = cfg.extra_trigger_size.get_rel() {
        cfg.extra_trigger_size = NumOrRelative::Num((max_size.0 as f64 * r) as i32);
    };
    cfg.margins.iter_mut().for_each(|(e, n)| {
        if let Ok(r) = n.get_rel() {
            *n = match e {
                Edge::Left | Edge::Right => NumOrRelative::Num((r * max_size_raw.0 as f64) as i32),
                Edge::Top | Edge::Bottom => NumOrRelative::Num((r * max_size_raw.1 as f64) as i32),
                _ => unreachable!(),
            };
        };
    });

    // remember to check height since we didn't do it in `parse_config`
    // when passing only `rel_height`
    if cfg.width.get_num().unwrap() * 2. > cfg.height.get_num().unwrap() {
        panic!(
            "relative height detect: width * 2 must be <= height: {} * 2 <= {}",
            cfg.width.get_num().unwrap(),
            cfg.height.get_num().unwrap()
        );
    }
}

pub trait WindowInitializer {
    fn init_window(app: &Application, cfgs: GroupConfig);
}

struct ButtonItem {
    cfg: Config,
    monitor: Monitor,
}

fn create_buttons(app: &gtk::Application, button_items: Vec<ButtonItem>) {
    button_items.into_iter().for_each(|bti| {
        println!("config: {:#?}", bti.cfg);
        let window = ui::new_window(app, bti.cfg);
        window.set_monitor(&bti.monitor);
        window.set_namespace("way-edges-widget");
    });
}
