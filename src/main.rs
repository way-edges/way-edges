mod config;
mod data;
mod ui;

use std::collections::HashMap;

use config::Config;
use gio::prelude::*;
use gtk4_layer_shell::Edge;
use ui::EventMap;

// https://github.com/wmww/gtk-layer-shell/blob/master/examples/simple-example.c
fn activate(application: &gtk::Application, cfgs: Vec<Config>) {
    cfgs.into_iter().for_each(|cfg| {
        ui::new_window(application, cfg);
    });
}

fn main() {
    std::env::set_var("GSK_RENDERER", "cairo");
    let application = gtk::Application::new(Some("sh.wmww.gtk-layer-example"), Default::default());

    application.connect_activate(|app| {
        let cfgs = vec![
            Config {
                edge: Edge::Left,
                position: Some(Edge::Bottom),
                size: (data::RADIUS, data::LENGTH),
                event_map: get_event_map_test(),
            },
            Config {
                edge: Edge::Bottom,
                position: Some(Edge::Left),
                size: (data::RADIUS, data::LENGTH),
                event_map: get_event_map_test(),
            },
            Config {
                edge: Edge::Top,
                // position: Some(Edge::Right),
                position: None,
                size: (data::RADIUS, data::LENGTH),
                event_map: get_event_map_test(),
            },
            Config {
                edge: Edge::Right,
                // position: Some(Edge::Top),
                position: None,
                size: (data::RADIUS, data::LENGTH),
                event_map: get_event_map_test(),
            },
        ];
        activate(app, cfgs);
    });

    application.run();
    // application.quit();
}

fn get_event_map_test() -> EventMap {
    let test_fn: Box<dyn Fn()> = Box::new(|| println!("test"));
    HashMap::from([(gtk::gdk::BUTTON_PRIMARY, test_fn)])
}
