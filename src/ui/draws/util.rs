use std::f64::consts::PI;

use crate::ui::draws::frame_manager::FrameManager;

use clap::error::Result;
use gtk::cairo;
use gtk::cairo::{Context, Format, ImageSurface, RectangleInt, Region};
use gtk::prelude::*;
use gtk::DrawingArea;
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
    edge: Edge,
    range: (f64, f64),
    extra_trigger_size: f64,
) -> impl FnMut(&Context, f64) {
    let offset: f64 = match edge {
        Edge::Right | Edge::Bottom => extra_trigger_size,
        _ => 0.,
    };
    move |ctx: &Context, visible_y: f64| {
        ctx.translate(-range.1 + visible_y - offset, 0.);
    }
}

pub fn draw_motion_now(
    ctx: &Context,
    visible_y: f64,
    edge: Edge,
    range: (f64, f64),
    extra_trigger_size: f64,
) {
    let offset: f64 = match edge {
        Edge::Right | Edge::Bottom => extra_trigger_size,
        _ => 0.,
    };
    ctx.translate(-range.1 + visible_y - offset, 0.);
}
use gtk::glib;

pub fn draw_frame_manager(
    frame_rate: u32,
    darea: &DrawingArea,
    window: &gtk::ApplicationWindow,
) -> impl FnMut(f64) -> Result<(), String> {
    let mut frame_manager = FrameManager::new(
        frame_rate,
        glib::clone!(
            #[weak]
            darea,
            move || {
                darea.queue_draw();
            }
        ),
    );
    move |visible_y: f64| {
        if transition_state::is_in_transition(visible_y) {
            frame_manager.start()?;
        } else {
            frame_manager.stop()?;
        }
        Ok(())
    }
}

pub fn draw_input_region(
    size: (f64, f64),
    edge: Edge,
    extra_trigger_size: f64,
) -> impl Fn(&gtk::ApplicationWindow, f64) -> Result<(), String> {
    let get_region: Box<dyn Fn(f64) -> Region> = match edge {
        Edge::Left => Box::new(move |visible_y: f64| {
            Region::create_rectangle(&RectangleInt::new(
                0,
                0,
                (visible_y + extra_trigger_size) as i32,
                size.1 as i32,
            ))
        }),
        Edge::Right => Box::new(move |visible_y: f64| {
            Region::create_rectangle(&RectangleInt::new(
                (size.0 - visible_y) as i32,
                0,
                (visible_y + extra_trigger_size).ceil() as i32,
                size.1 as i32,
            ))
        }),
        Edge::Top => Box::new(move |visible_y: f64| {
            Region::create_rectangle(&RectangleInt::new(
                0,
                0,
                size.1 as i32,
                (visible_y + extra_trigger_size) as i32,
            ))
        }),
        Edge::Bottom => Box::new(move |visible_y: f64| {
            Region::create_rectangle(&RectangleInt::new(
                0,
                (size.0 - visible_y) as i32,
                size.1 as i32,
                (visible_y + extra_trigger_size).ceil() as i32,
            ))
        }),
        _ => unreachable!(),
    };
    move |window: &gtk::ApplicationWindow, visible_y: f64| {
        window
            .surface()
            .ok_or("Input region surface not found")?
            .set_input_region(&get_region(visible_y));
        Ok(())
    }
}

pub fn draw_rotation(edge: Edge, size: (f64, f64)) -> Box<dyn Fn(&Context)> {
    match edge {
        Edge::Left => Box::new(move |_: &Context| {}),
        Edge::Right => Box::new(move |ctx: &Context| {
            ctx.rotate(180_f64.to_radians());
            ctx.translate(-size.0, -size.1);
        }),
        Edge::Top => Box::new(move |ctx: &Context| {
            ctx.rotate(90.0_f64.to_radians());
            ctx.translate(0., -size.1);
        }),
        Edge::Bottom => Box::new(move |ctx: &Context| {
            ctx.rotate(270.0_f64.to_radians());
            ctx.translate(-size.0, 0.);
        }),
        _ => unreachable!(),
    }
}

pub fn draw_input_region_now(
    window: &gtk::ApplicationWindow,
    visible_y: f64,
    size: (f64, f64),
    edge: Edge,
    extra_trigger_size: f64,
) -> Result<(), String> {
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
    window
        .surface()
        .ok_or("Input region surface not found")?
        .set_input_region(&region);
    Ok(())
}

pub fn draw_rotation_now(ctx: &Context, edge: Edge, size: (f64, f64)) {
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
