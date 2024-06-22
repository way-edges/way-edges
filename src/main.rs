mod config;
mod data;
mod ui;

use std::collections::HashMap;

use config::Config;
use gio::prelude::*;
use gtk4_layer_shell::Edge;
use ui::EventMap;

// https://github.com/wmww/gtk-layer-shell/blob/master/examples/simple-example.c
fn activate(application: &gtk::Application) {
    let test_fn: Box<dyn Fn()> = Box::new(|| println!("test"));
    let event_map: EventMap = HashMap::from([(gtk::gdk::BUTTON_PRIMARY, test_fn)]);
    ui::new_window(
        application,
        // (data::RADIUS, data::LENGTH),
        Config {
            edge: Edge::Left,
            position: Some(Edge::Bottom),
            size: (data::RADIUS, data::LENGTH),
            event_map,
        },
    );

    let test_fn: Box<dyn Fn()> = Box::new(|| println!("test"));
    let event_map: EventMap = HashMap::from([(gtk::gdk::BUTTON_PRIMARY, test_fn)]);
    ui::new_window(
        application,
        // (data::RADIUS, data::LENGTH),
        Config {
            edge: Edge::Top,
            position: None,
            size: (data::RADIUS, data::LENGTH),
            event_map,
        },
    );
}

fn main() {
    std::env::set_var("GSK_RENDERER", "cairo");
    let application = gtk::Application::new(Some("sh.wmww.gtk-layer-example"), Default::default());

    application.connect_activate(|app| {
        activate(app);
    });

    application.run();
    // application.quit();
}
