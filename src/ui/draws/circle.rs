#![allow(dead_code)]
use std::f64::consts::PI;

use gtk::{
    cairo::{Context, RadialGradient},
    gdk::RGBA,
};

/// do not use `PI`
pub fn draw_fan(ctx: &Context, point: (f64, f64), radius: f64, start: f64, end: f64) {
    ctx.arc(point.0, point.1, radius, start * PI, end * PI);
    ctx.line_to(point.0, point.1);
    ctx.close_path();
}

/// do not use `PI`
pub fn draw_fan_no_close(ctx: &Context, point: (f64, f64), radius: f64, start: f64, end: f64) {
    ctx.arc(point.0, point.1, radius, start * PI, end * PI);
    // ctx.line_to(point.0, point.1);
    // ctx.close_path();
}

pub fn gen_radial_grandient(
    point: (f64, f64),
    radius: f64,
    start_color: RGBA,
    end_color: RGBA,
) -> RadialGradient {
    let rg = RadialGradient::new(point.0, point.1, 0., point.0, point.1, radius);
    rg.add_color_stop_rgba(
        0.,
        start_color.red().into(),
        start_color.green().into(),
        start_color.blue().into(),
        start_color.alpha().into(),
    );
    rg.add_color_stop_rgba(
        1.,
        end_color.red().into(),
        end_color.green().into(),
        end_color.blue().into(),
        end_color.alpha().into(),
    );
    rg
}
