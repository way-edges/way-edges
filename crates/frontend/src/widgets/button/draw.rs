use std::cell::Cell;
use std::rc::Rc;

use crate::window::WindowContext;

use super::BtnConfig;
use gtk::cairo::Context;
use gtk::prelude::*;
use gtk4_layer_shell::Edge;

pub fn setup_draw(
    window: &mut WindowContext,
    btn_conf: &BtnConfig,
    edge: Edge,
    pressing_state: Rc<Cell<bool>>,
) {
    let darw_func = make_draw_func(btn_conf, edge, pressing_state);
    window.set_draw_func(Some(move || Some(darw_func())));
}

use std::f64::consts::PI;

use gtk::cairo::{self, ImageSurface, LinearGradient};
use gtk::gdk::RGBA;
use util::draw::new_surface;
use util::Z;

struct DrawConfig {
    size: (i32, i32),
    color: RGBA,
    border_width: i32,
    border_color: RGBA,
}
impl DrawConfig {
    fn new_surf(&self) -> (ImageSurface, Context) {
        let surf = new_surface(self.size);
        let ctx = cairo::Context::new(&surf).unwrap();
        (surf, ctx)
    }
}

fn draw_top(conf: &DrawConfig, pressing: bool) -> ImageSurface {
    let (surf, ctx) = conf.new_surf();

    let size = (conf.size.0 as f64, conf.size.1 as f64);
    let border_width = conf.border_width as f64;

    // path
    let radius = size.1 - (border_width / 2.);
    ctx.arc(size.1, Z, radius, 1. * PI, 1.5 * PI);
    ctx.move_to(size.1, radius);
    ctx.line_to(size.0 - size.1, radius);
    ctx.arc(size.0 - size.1, Z, radius, 1.5 * PI, 2. * PI);
    ctx.close_path();

    // content
    ctx.set_source_color(&conf.color);
    ctx.fill_preserve().unwrap();

    // mask
    let lg = LinearGradient::new(Z, size.0, size.1, Z);
    lg.add_color_stop_rgba(0., 0., 0., 0., 0.);
    lg.add_color_stop_rgba(0.4, 0., 0., 0., 0.);
    lg.add_color_stop_rgba(1., 0., 0., 0., 0.7);
    ctx.set_source(&lg).unwrap();
    ctx.fill_preserve().unwrap();

    if pressing {
        ctx.set_source_color(&RGBA::new(0., 0., 0., 0.3));
        ctx.fill_preserve().unwrap();
    }

    // border
    ctx.set_source_color(&conf.border_color);
    ctx.set_line_width(border_width);
    ctx.stroke_preserve().unwrap();

    surf
}

fn draw_right(conf: &DrawConfig, pressing: bool) -> ImageSurface {
    let (surf, ctx) = conf.new_surf();
    let base = draw_top(conf, pressing);

    ctx.rotate(90.0_f64.to_radians());
    ctx.set_source_surface(&base, 0., 0.).unwrap();
    ctx.paint().unwrap();

    surf
}

fn draw_bottom(conf: &DrawConfig, pressing: bool) -> ImageSurface {
    let (surf, ctx) = conf.new_surf();
    let base = draw_top(conf, pressing);

    ctx.rotate(180.0_f64.to_radians());
    ctx.translate(-conf.size.0 as f64, -conf.size.1 as f64);
    ctx.set_source_surface(&base, 0., 0.).unwrap();
    ctx.paint().unwrap();

    surf
}

fn draw_left(conf: &DrawConfig, pressing: bool) -> ImageSurface {
    let (surf, ctx) = conf.new_surf();
    let base = draw_top(conf, pressing);

    ctx.rotate(-90.0_f64.to_radians());
    ctx.translate(-conf.size.0 as f64, Z);
    ctx.set_source_surface(&base, 0., 0.).unwrap();
    ctx.paint().unwrap();

    surf
}

fn make_draw_func(
    btn_conf: &BtnConfig,
    edge: Edge,
    pressing_state: Rc<Cell<bool>>,
) -> impl Fn() -> ImageSurface {
    let func = match edge {
        Edge::Left => draw_left,
        Edge::Right => draw_right,
        Edge::Top => draw_top,
        Edge::Bottom => draw_bottom,
        _ => unreachable!(),
    };

    let content_size = &btn_conf.size;
    let border_width = btn_conf.border_width;
    let size = match edge {
        Edge::Left | Edge::Right => (
            content_size.thickness.get_num().unwrap().ceil() as i32 + border_width,
            content_size.length.get_num().unwrap().ceil() as i32 + border_width * 2,
        ),
        Edge::Top | Edge::Bottom => (
            content_size.length.get_num().unwrap().ceil() as i32 + border_width * 2,
            content_size.thickness.get_num().unwrap().ceil() as i32 + border_width,
        ),
        _ => unreachable!(),
    };
    let draw_conf = DrawConfig {
        size,
        border_width,
        color: btn_conf.color,
        border_color: btn_conf.border_color,
    };

    move || func(&draw_conf, pressing_state.get())
}
