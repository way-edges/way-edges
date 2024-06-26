use crate::config::{Config, MonitorSpecifier};
use crate::ui;
use gio::prelude::*;
use gtk::gdk::Monitor;
use gtk::prelude::{DisplayExt, MonitorExt};
use gtk4_layer_shell::LayerShell;

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

pub fn activate(application: &gtk::Application, cfgs: Vec<Config>) {
    let dt_display = gtk::gdk::Display::default();
    if let Some(t) = dt_display {
        cfgs.into_iter().for_each(|cfg| {
            let monitor = find_monitor(&t.monitors(), cfg.monitor.clone());
            let window = ui::new_window(application, cfg);
            window.set_monitor(&monitor);
            window.set_namespace("way-edges-widget");
        });
    } else {
        panic!("display for monitor not found");
    };
}
