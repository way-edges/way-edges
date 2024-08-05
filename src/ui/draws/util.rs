use std::f64::consts::PI;

use crate::ui::draws::frame_manager::FrameManager;

use educe::Educe;
use gtk::cairo::{self, Context, Format, ImageSurface, RectangleInt, Region};
use gtk::gdk::RGBA;
use gtk::pango::Layout;
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

#[derive(Educe, Clone)]
#[educe(Debug)]
pub struct ImageData {
    pub width: i32,
    pub height: i32,
    pub stride: i32,
    pub format: Format,
    #[educe(Debug(ignore))]
    pub data: Vec<u8>,
}
unsafe impl Send for ImageData {}
impl ImageData {
    pub unsafe fn temp_surface(&mut self) -> ImageSurface {
        ImageSurface::create_for_data_unsafe(
            self.data.as_ptr() as *mut _,
            self.format,
            self.width,
            self.height,
            self.stride,
        )
        .unwrap()
    }
}
impl From<ImageSurface> for ImageData {
    fn from(value: ImageSurface) -> Self {
        Self {
            width: value.width(),
            height: value.height(),
            stride: value.stride(),
            format: value.format(),
            data: value.take_data().unwrap().to_vec(),
        }
    }
}
impl From<ImageData> for ImageSurface {
    fn from(value: ImageData) -> Self {
        ImageSurface::create_for_data(
            value.data,
            value.format,
            value.width,
            value.height,
            value.stride,
        )
        .unwrap()
    }
}

pub fn draw_text(pl: &Layout, color: &RGBA, text: &str) -> ImageSurface {
    pl.set_text(text);
    let size = pl.pixel_size();
    let surf = new_surface(size);
    let ctx = cairo::Context::new(&surf).unwrap();
    ctx.set_antialias(cairo::Antialias::None);
    ctx.set_source_color(color);
    pangocairo::functions::show_layout(&ctx, pl);
    surf
}
