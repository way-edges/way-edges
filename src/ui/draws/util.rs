use std::f64::consts::PI;

use educe::Educe;
use gtk::cairo::{self, Context, Format, ImageSurface, RectangleInt, Region};
use gtk::gdk::RGBA;
use gtk::pango::Layout;
use gtk::prelude::*;
use gtk4_layer_shell::Edge;

pub const Z: f64 = 0.;

pub fn from_angel(a: f64) -> f64 {
    a / 180. * PI
}

// pub fn copy_surface(src: &ImageSurface) -> ImageSurface {
//     let dst = ImageSurface::create(Format::ARgb32, src.width(), src.height()).unwrap();
//     let ctx = cairo::Context::new(&dst).unwrap();
//     copy_surface_to_context(&ctx, src);
//     dst
// }
//
// pub fn copy_surface_to_context(dst: &Context, src: &ImageSurface) {
//     dst.set_source_surface(src, Z, Z).unwrap();
//     dst.rectangle(Z, Z, src.width().into(), src.height().into());
//     dst.fill().unwrap();
// }

pub fn new_surface(size: (i32, i32)) -> ImageSurface {
    ImageSurface::create(Format::ARgb32, size.0, size.1).unwrap()
}

pub fn draw_motion(ctx: &Context, visible_y: f64, range: (f64, f64)) {
    ctx.translate(-range.1 + visible_y, 0.);
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
) -> Region {
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
                (size.0 - visible_y - extra_trigger_size) as i32,
                size.1 as i32,
                (visible_y + extra_trigger_size).ceil() as i32,
            ),
            _ => unreachable!(),
        };
        Region::create_rectangle(&RectangleInt::new(x, y, w, h))
    };
    window.surface().unwrap().set_input_region(&region);
    region
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
    /// you should not mutate this
    pub unsafe fn temp_surface(&self) -> ImageSurface {
        ImageSurface::create_for_data_unsafe(
            self.data.as_ptr() as *const _ as *mut _,
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
    let (ink, _) = pl.pixel_extents();
    let surf = new_surface((ink.width(), ink.height()));
    let ctx = cairo::Context::new(&surf).unwrap();
    ctx.set_source_color(color);
    ctx.move_to(Z, -ink.y() as f64);
    pangocairo::functions::show_layout(&ctx, pl);
    surf
}

pub fn draw_text_to_size(pl: &Layout, color: &RGBA, text: &str, height: i32) -> ImageSurface {
    pl.set_text(text);
    let (ink, logic) = pl.pixel_extents();
    let scale = height as f64 / (logic.height() - ink.y()) as f64;

    let surf = new_surface(((ink.width() as f64 * scale).ceil() as i32, ink.height()));
    let ctx = cairo::Context::new(&surf).unwrap();
    ctx.set_source_color(color);
    ctx.scale(scale, scale);
    ctx.move_to(Z, -ink.y() as f64);
    pangocairo::functions::show_layout(&ctx, pl);
    surf
}

pub fn combine_2_image_vertical_left(
    img1: &ImageSurface,
    img2: &ImageSurface,
    gap: Option<i32>,
) -> ImageSurface {
    let gap = gap.unwrap_or(0);
    let surf = ImageSurface::create(
        Format::ARgb32,
        img1.width().max(img2.width()),
        img1.height() + img2.height() + gap,
    )
    .unwrap();
    let ctx = cairo::Context::new(&surf).unwrap();
    ctx.set_source_surface(img1, Z, Z).unwrap();
    ctx.paint().unwrap();
    ctx.translate(Z, (img1.height() + gap) as f64);
    ctx.set_source_surface(img2, Z, Z).unwrap();
    ctx.paint().unwrap();

    surf
}

// pub fn combine_vertcal_left(imgs: &[ImageSurface], gap: Option<i32>) -> ImageSurface {
//     let last_index = imgs.len() - 1;
//
//     let mut max_width = 0;
//     let mut total_height = 0;
//     imgs.iter().enumerate().for_each(|(index, img)| {
//         max_width = max_width.max(img.width());
//         total_height += img.height();
//
//         // count in gap
//         if index != last_index {
//             if let Some(gap) = gap {
//                 total_height += gap;
//             }
//         }
//     });
//
//     let surf = new_surface((max_width, total_height));
//     let ctx = Context::new(&surf).unwrap();
//
//     imgs.iter().enumerate().for_each(|(index, img)| {
//         ctx.set_source_surface(img, Z, Z).unwrap();
//         ctx.paint().unwrap();
//         ctx.translate(Z, img.height() as f64);
//
//         // translate for gap
//         if index != last_index {
//             if let Some(gap) = gap {
//                 ctx.translate(gap as f64, Z);
//             }
//         }
//     });
//
//     surf
// }

pub fn combine_horizonal_center(imgs: &[ImageSurface], gap: Option<i32>) -> ImageSurface {
    let last_index = imgs.len() - 1;

    let mut max_height = 0;
    let mut total_width = 0;
    imgs.iter().enumerate().for_each(|(index, img)| {
        max_height = max_height.max(img.height());
        total_width += img.width();

        // count in gap
        if index != last_index {
            if let Some(gap) = gap {
                total_width += gap;
            }
        }
    });

    let surf = new_surface((total_width, max_height));
    let ctx = Context::new(&surf).unwrap();

    imgs.iter().enumerate().for_each(|(index, img)| {
        let height = img.height();
        let y = (max_height - height) / 2;
        ctx.set_source_surface(img, Z, y as f64).unwrap();
        ctx.paint().unwrap();
        ctx.translate(img.width() as f64, Z);

        // translate for gap
        if index != last_index {
            if let Some(gap) = gap {
                ctx.translate(gap as f64, Z);
            }
        }
    });

    surf
}

pub fn color_transition(start_color: RGBA, stop_color: RGBA, v: f32) -> RGBA {
    let r = start_color.red() + (stop_color.red() - start_color.red()) * v;
    let g = start_color.green() + (stop_color.green() - start_color.green()) * v;
    let b = start_color.blue() + (stop_color.blue() - start_color.blue()) * v;
    let a = start_color.alpha() + (stop_color.alpha() - start_color.alpha()) * v;
    RGBA::new(r, g, b, a)
}

pub fn color_mix(one: RGBA, two: RGBA) -> RGBA {
    let a = 1. - (1. - one.alpha()) * (1. - two.alpha());
    let r = (one.red() * one.alpha() + two.red() * two.alpha() * (1. - one.alpha())) / a;
    let g = (one.green() * one.alpha() + two.green() * two.alpha() * (1. - one.alpha())) / a;
    let b = (one.blue() * one.alpha() + two.blue() * two.alpha() * (1. - one.alpha())) / a;
    RGBA::new(r, g, b, a)
}
