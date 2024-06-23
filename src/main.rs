mod config;
mod data;
mod ui;

use core::panic;
use std::{collections::HashMap, thread, time::Duration};

use config::{Config, MonitorSpecifier};
use gio::prelude::*;
use gtk::gdk::Monitor;
use gtk::prelude::{DisplayExt, MonitorExt};
use gtk4_layer_shell::{Edge, LayerShell};
use ui::EventMap;

fn find_monitor(monitors: &gio::ListModel, specifier: MonitorSpecifier) -> Monitor {
    let a = match specifier {
        config::MonitorSpecifier::ID(index) => {
            let a = monitors
                .iter::<Monitor>()
                .nth(index)
                .unwrap_or_else(|| panic!("error matching monitor with id: {index}"))
                .unwrap_or_else(|_| panic!("error matching monitor with id: {index}"));
            a
        }
        config::MonitorSpecifier::Name(name) => {
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

fn activate(application: &gtk::Application, cfgs: Vec<Config>) {
    let dt_display = gtk::gdk::Display::default();
    if let Some(t) = dt_display {
        cfgs.into_iter().for_each(|cfg| {
            let monitor = find_monitor(&t.monitors(), cfg.monitor.clone());
            let window = ui::new_window(application, cfg);
            window.set_monitor(&monitor);
        });
    } else {
        panic!("display for monitor not found");
    };
}

fn main() {
    std::env::set_var("GSK_RENDERER", "cairo");
    // return;

    let application = gtk::Application::new(Some("sh.wmww.gtk-layer-example"), Default::default());

    application.connect_activate(|app| {
        let group_map = config::get_config().unwrap();
        if group_map.is_empty() {
            panic!("empty config");
        }
        let cfgs = if group_map.len() == 1 {
            group_map.into_values().last().unwrap()
        } else {
            unreachable!()
        };
        cfgs.iter().for_each(|c| {
            println!("{}", c.debug());
        });

        activate(app, cfgs);
    });

    application.run();

    loop {
        thread::sleep(Duration::from_millis(1000));
    }
}

fn get_event_map_test() -> EventMap {
    let test_fn: Box<dyn Fn()> = Box::new(|| println!("test"));
    HashMap::from([(gtk::gdk::BUTTON_PRIMARY, test_fn)])
}
