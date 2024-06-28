pub mod default;
pub mod hyprland;

use crate::config::{Config, GroupConfig, MonitorSpecifier};
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

fn calculate_height(cfg: &mut Config, max_size: (i32, i32)) {
    let mx = match cfg.edge {
        Edge::Left | Edge::Right => max_size.0,
        Edge::Top | Edge::Bottom => max_size.1,
        _ => unreachable!(),
    };
    cfg.size.1 = mx as f64 * cfg.rel_height;
    // remember to check height since we didn't do it in `parse_config`
    // when passing only `rel_height`
    if cfg.size.0 * 2. > cfg.size.1 {
        panic!("relative height detect: width * 2 must be <= height: {{cfg.size.0}} * 2 <= {{cfg.size.1}}");
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
        // match relative height
        let window = ui::new_window(app, bti.cfg);
        window.set_monitor(&bti.monitor);
        window.set_namespace("way-edges-widget");
    });
}
