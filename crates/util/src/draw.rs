use cairo::{Format, ImageSurface};
use gtk::gdk::RGBA;

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
