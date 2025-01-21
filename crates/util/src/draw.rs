use std::f64::consts::PI;

use cairo::{Format, ImageSurface, Path};
use gdk::{pango::Layout, prelude::GdkCairoContextExt, RGBA};

use crate::Z;

pub fn new_surface(size: (i32, i32)) -> ImageSurface {
    ImageSurface::create(Format::ARgb32, size.0, size.1).unwrap()
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

pub fn draw_rect_path(radius: f64, size: (f64, f64), corners: [bool; 4]) -> Result<Path, String> {
    let surf =
        cairo::ImageSurface::create(Format::ARgb32, size.0.ceil() as i32, size.1.ceil() as i32)
            .unwrap();
    let ctx = cairo::Context::new(&surf).unwrap();

    // draw
    {
        // top left corner
        {
            ctx.move_to(Z, radius);
            if corners[0] {
                let center = (radius, radius);
                ctx.arc(center.0, center.1, radius, PI, 1.5 * PI);
            } else {
                ctx.line_to(Z, Z);
            }
            let x = size.0 - radius;
            let y = Z;
            ctx.line_to(x, y);
        }

        // top right corner
        {
            if corners[1] {
                let center = (size.0 - radius, radius);
                ctx.arc(center.0, center.1, radius, 1.5 * PI, 2. * PI);
            } else {
                ctx.line_to(size.0, Z);
            }
            let x = size.0;
            let y = size.1 - radius;
            ctx.line_to(x, y);
        }

        // bottom right corner
        {
            if corners[2] {
                let center = (size.0 - radius, size.1 - radius);
                ctx.arc(center.0, center.1, radius, 0., 0.5 * PI);
            } else {
                ctx.line_to(size.0, size.1);
            }
            let x = radius;
            let y = size.1;
            ctx.line_to(x, y);
        }

        // bottom left corner
        {
            if corners[3] {
                let center = (radius, size.1 - radius);
                ctx.arc(center.0, center.1, radius, 0.5 * PI, PI);
            } else {
                ctx.line_to(Z, size.1);
            }
            let x = Z;
            let y = radius;
            ctx.line_to(x, y);
        }

        ctx.close_path();
        Ok(ctx.copy_path().unwrap())
    }
}

pub fn draw_text_to_size(pl: &Layout, color: &RGBA, text: &str, height: i32) -> ImageSurface {
    if text.is_empty() {
        return new_surface((0, 0));
    }

    pl.set_text(text);
    let (_, logic) = pl.pixel_extents();
    let line_num = text.lines().count();
    let one_line_height = logic.height() / line_num as i32;
    let scale = height as f64 / one_line_height as f64;

    let size = (
        (logic.width() as f64 * scale).ceil() as i32,
        height * line_num as i32,
    );
    let surf = new_surface(size);
    let ctx = cairo::Context::new(&surf).unwrap();
    ctx.set_source_color(color);
    ctx.scale(scale, scale);
    pangocairo::functions::show_layout(&ctx, pl);
    surf
}

pub fn draw_fan(ctx: &cairo::Context, point: (f64, f64), radius: f64, start: f64, end: f64) {
    ctx.arc(point.0, point.1, radius, start * PI, end * PI);
    ctx.line_to(point.0, point.1);
    ctx.close_path();
}
