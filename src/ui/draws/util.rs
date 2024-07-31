use std::f64::consts::PI;

use crate::ui::draws::frame_manager::FrameManager;

use gtk::cairo::{self, Context, Format, ImageSurface, RectangleInt, Region};
use gtk::prelude::*;
use gtk4_layer_shell::Edge;

use super::transition_state::{self};

pub const Z: f64 = 0.;

pub fn from_angel(a: f64) -> f64 {
    a / 180. * PI
}

pub fn copy_surface(src: &ImageSurface) -> ImageSurface {
    let dst = ImageSurface::create(Format::ARgb32, src.width(), src.height()).unwrap();
    let ctx = cairo::Context::new(&dst).unwrap();
    copy_surface_to_context(&ctx, src);
    dst
}

pub fn copy_surface_to_context(dst: &Context, src: &ImageSurface) {
    dst.set_source_surface(src, Z, Z).unwrap();
    dst.rectangle(Z, Z, src.width().into(), src.height().into());
    dst.fill().unwrap();
}

pub fn new_surface(size: (i32, i32)) -> ImageSurface {
    ImageSurface::create(Format::ARgb32, size.0, size.1).unwrap()
}

pub fn draw_motion(
    ctx: &Context,
    visible_y: f64,
    edge: Edge,
    range: (f64, f64),
    extra_trigger_size: f64,
) {
    // let offset: f64 = match edge {
    //     Edge::Right | Edge::Bottom => extra_trigger_size,
    //     _ => 0.,
    // };
    // println!("offset: {}", offset);
    // ctx.translate(-range.1 + visible_y - offset, 0.);
    ctx.translate(-range.1 + visible_y, 0.);
}

pub fn ensure_frame_manager(frame_manager: &mut FrameManager, y: f64) {
    if transition_state::is_in_transition(y) {
        frame_manager.start().unwrap();
    } else {
        frame_manager.stop().unwrap();
    }
}

pub fn draw_rotation(ctx: &Context, edge: Edge, size: (f64, f64)) {
    match edge {
        Edge::Left => {}
        Edge::Right => {
            ctx.rotate(180_f64.to_radians());
            ctx.translate(-size.0, -size.1);
        }
        Edge::Top => {
            ctx.rotate(90.0_f64.to_radians());
            ctx.translate(0., -size.1);
        }
        Edge::Bottom => {
            ctx.rotate(270.0_f64.to_radians());
            ctx.translate(-size.0, 0.);
        }
        _ => unreachable!(),
    }
}

pub fn ensure_input_region(
    window: &gtk::ApplicationWindow,
    visible_y: f64,
    size: (f64, f64),
    edge: Edge,
    extra_trigger_size: f64,
) {
    let region = {
        let (x, y, w, h) = match edge {
            Edge::Left => (0, 0, (visible_y + extra_trigger_size) as i32, size.1 as i32),
            Edge::Right => (
                (size.0 - visible_y) as i32,
                0,
                (visible_y + extra_trigger_size).ceil() as i32,
                size.1 as i32,
            ),
            Edge::Top => (0, 0, size.1 as i32, (visible_y + extra_trigger_size) as i32),
            Edge::Bottom => (
                0,
                (size.0 - visible_y) as i32,
                size.1 as i32,
                (visible_y + extra_trigger_size).ceil() as i32,
            ),
            _ => unreachable!(),
        };
        Region::create_rectangle(&RectangleInt::new(x, y, w, h))
    };
    window.surface().unwrap().set_input_region(&region);
}
