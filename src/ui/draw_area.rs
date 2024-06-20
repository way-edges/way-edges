use super::draws;
use crate::data;
use gio::prelude::*;
use gtk::cairo::Context;
use gtk::cairo::LinearGradient;
use gtk::gdk::BUTTON_PRIMARY;
use gtk::gdk::{self, prelude::*, RGBA};
use gtk::glib;
use gtk::prelude::*;
use gtk::DrawingArea;
use gtk::GestureClick;
use std::cell::Cell;
use std::rc::Rc;

pub fn setup_draw(window: &gtk::ApplicationWindow, size: (f64, f64)) {
    let darea = DrawingArea::new();
    let map_size = (
        (size.0 as i32 + data::GLOW_SIZE as i32),
        (size.1 as i32 + data::GLOW_SIZE as i32),
    );
    let is_pressing = setup_event(&darea);
    darea.set_width_request(map_size.0);
    darea.set_height_request(map_size.1);
    let draw = make_draw_fn(map_size, size);
    darea.set_draw_func(move |area, context, width, height| {
        let is_pressing = is_pressing.get();
        draw(context, is_pressing.is_some());
    });
    window.set_child(Some(&darea));
}

fn make_draw_fn(map_size: (i32, i32), size: (f64, f64)) -> impl Fn(&Context, bool) {
    let (b, n, p) = draws::store::draw_to_surface(map_size, size);
    let f_map_size = (map_size.0 as f64, map_size.1 as f64);

    move |ctx: &Context, pressing: bool| {
        // base_surface
        ctx.set_source_surface(&b, 0., 0.).unwrap();
        ctx.rectangle(0., 0., f_map_size.0, f_map_size.1);
        ctx.fill().unwrap();

        // mask
        if pressing {
            ctx.set_source_surface(&p, 0., 0.).unwrap();
        } else {
            ctx.set_source_surface(&n, 0., 0.).unwrap();
        }
        ctx.rectangle(0., 0., f_map_size.0, f_map_size.1);
        ctx.fill().unwrap();
        // base_surf
    }
}

fn setup_event(darea: &DrawingArea) -> Rc<Cell<Option<u32>>> {
    let pressing_state = Rc::new(Cell::new(None));

    let left_click_control = GestureClick::builder().button(0).exclusive(true).build();
    println!(
        "primary: {}, secondary: {}, middle: {}",
        gdk::BUTTON_PRIMARY,
        gdk::BUTTON_SECONDARY,
        gdk::BUTTON_MIDDLE
    );
    left_click_control.connect_pressed(
        glib::clone!(@strong pressing_state, @weak darea => move |g, _, _, _| {
            println!("key: {}", g.current_button());
            pressing_state.set(Some(g.current_button()));
            darea.queue_draw();
        }),
    );
    left_click_control.connect_released(
        glib::clone!(@strong pressing_state, @weak darea => move |g, _, _, _| {
            pressing_state.set(None);
            darea.queue_draw();
        }),
    );
    // left_click_control.connect_unpaired_release(
    //     glib::clone!(@strong panel, @weak darea => move |g, _, _, d, _| {
    //         println!("{}", d);
    //         panel.borrow_mut().set_pressing_state(false);
    //         darea.queue_draw();
    //     }),
    // );
    darea.add_controller(left_click_control);
    pressing_state
}
