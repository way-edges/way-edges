use super::draws;
use crate::data;
use gio::prelude::*;
use gtk::cairo::Context;
use gtk::cairo::LinearGradient;
use gtk::gdk::{self, prelude::*, RGBA};
use gtk::glib;
use gtk::prelude::*;
use gtk::DrawingArea;
use gtk::EventController;
use gtk::GestureClick;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;

pub fn setup_draw(window: &gtk::ApplicationWindow, size: (f64, f64)) {
    let darea = DrawingArea::new();
    let map_size = (
        (size.0 as i32 + data::GLOW_SIZE as i32),
        (size.1 as i32 + data::GLOW_SIZE as i32),
    );
    let panel = Rc::new(RefCell::new(draws::store::PanelState::new(map_size, size)));
    let click_controller = GestureClick::new();
    click_controller.set_button(gdk::BUTTON_PRIMARY);
    click_controller.connect_pressed(
        glib::clone!(@strong panel, @weak darea => move |g, _, _, _| {
            println!("pressing");
            panel.borrow_mut().set_pressing_state(true);
            darea.queue_draw();
        }),
    );
    click_controller.connect_released(
        glib::clone!(@strong panel, @weak darea => move |g, _, _, _| {
            println!("pressing");
            panel.borrow_mut().set_pressing_state(false);
            darea.queue_draw();
        }),
    );
    darea.add_controller(click_controller);
    darea.set_width_request(map_size.0);
    darea.set_height_request(map_size.1);
    darea.set_draw_func(move |area, context, width, height| {
        panel.borrow_mut().draw_into_surface(context);
    });
    window.set_child(Some(&darea));
}
