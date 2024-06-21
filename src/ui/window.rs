use super::draw_area;
use gtk::{
    cairo::{RectangleInt, Region},
    prelude::*,
    Application, CssProvider, STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use interval_task::runner::ExternalRunnerExt;

use super::draw_area::EventMap;
use gio::prelude::*;
use gtk::cairo::Context;
use gtk::cairo::LinearGradient;
use gtk::gdk::BUTTON_PRIMARY;
use gtk::gdk::{self, prelude::*, RGBA};
use gtk::glib;
use gtk::{DrawingArea, GestureClick};
use std::cell::Cell;
use std::collections::HashMap;
use std::ops::{Add, Mul, Sub};
use std::rc::Rc;
use std::time::{Duration, Instant};

pub fn new_window(app: &Application, size: (f64, f64)) {
    // Create a normal GTK window however you like
    let window = gtk::ApplicationWindow::new(app);

    window.init_layer_shell();

    window.set_layer(Layer::Top);

    // Push other windows out of the way
    // window.auto_exclusive_zone_enable();

    let anchors = [
        (Edge::Left, true),
        (Edge::Right, false),
        (Edge::Top, false),
        (Edge::Bottom, true),
    ];
    for (anchor, state) in anchors {
        window.set_anchor(anchor, state);
    }

    let test_fn: Box<dyn Fn()> = Box::new(|| println!("test"));
    let event_map: EventMap = HashMap::from([(BUTTON_PRIMARY, test_fn)]);
    let darea = draw_area::setup_draw(&window, size, event_map);

    window.connect_show(
        // glib::clone!(@weak darea => move |w: &gtk::ApplicationWindow| {
        move |w: &gtk::ApplicationWindow| {
            // set input region
            w.surface()
                .expect("Surface not detected")
                .set_input_region(&Region::create_rectangle(&RectangleInt::new(
                    0,
                    0,
                    size.0 as i32,
                    size.1 as i32,
                )));

            // transparency background !! may not work for some gtk4 theme, and idk how to fix !!
            let provider = CssProvider::new();
            provider
                // .load_from_string("window.background { background: unset; border: 1px solid white; }");
                .load_from_string("window.background { background: unset; }");
            gtk::style_context_add_provider_for_display(
                &WidgetExt::display(w),
                &provider,
                STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        },
    );

    window.present();
}
