use cairo::Path;
use config::widgets::slide::base::{Direction, SlideConfig};
use gtk::cairo::Context;
use gtk::{pango, prelude::*};
use gtk4_layer_shell::Edge;

use std::f64::consts::PI;

use gtk::cairo::{self, ImageSurface, LinearGradient};
use gtk::gdk::RGBA;
use util::draw::new_surface;
use util::Z;

use super::font::get_font_face;

struct DrawConfig {
    size: (i32, i32),
    border_width: i32,

    obtuse_angle: f64,
    radius: f64,

    bg_color: RGBA,
    fg_color: RGBA,
    border_color: RGBA,
    text_color: RGBA,
    is_text_position_start: bool,
    progress_direction: Direction,
}
impl DrawConfig {
    fn new(slide_conf: &SlideConfig) -> Self {
        let content_size = slide_conf.size().unwrap();
        let size = (content_size.1.ceil() as i32, content_size.0.ceil() as i32);
        Self {
            size,
            border_width: slide_conf.border_width,
            obtuse_angle: slide_conf.obtuse_angle,
            radius: slide_conf.radius,
            bg_color: slide_conf.bg_color,
            fg_color: slide_conf.fg_color,
            border_color: slide_conf.border_color,
            text_color: slide_conf.text_color,
            is_text_position_start: slide_conf.is_text_position_start,
            progress_direction: slide_conf.progress_direction,
        }
    }
    fn new_horizontal_surf(&self) -> (ImageSurface, Context) {
        let surf = new_surface(self.size);
        let ctx = cairo::Context::new(&surf).unwrap();
        (surf, ctx)
    }
    fn new_vertical_surf(&self) -> (ImageSurface, Context) {
        let surf = new_surface((self.size.1, self.size.0));
        let ctx = cairo::Context::new(&surf).unwrap();
        (surf, ctx)
    }
}

fn draw_text(progress: f64) -> ImageSurface {
    let font_face = get_font_face().unwrap();
    let desc = font_face.();
    let pc = pangocairo::pango::Context::new();
    let fm = pangocairo::FontMap::default();
    pc.set_font_map(Some(&fm));
    let mut desc = pc.font_description().unwrap();
    desc.set_absolute_size(font_size * 1024.);
    if let Some(font_family) = font_family {
        desc.set_family(font_family.as_str());
    }
    pc.set_font_description(Some(&desc));
    let layout = pangocairo::pango::Layout::new(&pc);
    pango::Layou
}

fn draw_slide_path(
    obtuse_angle: f64,
    radius: f64,
    size: (f64, f64),
    close_path: bool,
) -> Result<(Path, f64), String> {
    let ctx =
        cairo::Context::new(new_surface((size.0.ceil() as i32, size.1.ceil() as i32))).unwrap();

    struct BigTrangle {
        height: f64,
        width: f64,
        left_angle: f64,
        top_angle: f64,
    }

    struct TrangleForRotation {
        rotate_angle: f64,
        height: f64,
        width: f64,
        max_line: f64,
    }

    fn from_angle(a: f64) -> f64 {
        a / 180. * PI
    }

    // get trangle for rotation
    let rotate_angle = 180. - obtuse_angle;
    let height = size.1;
    let width = size.1 / from_angle(rotate_angle).tan();
    let max_line = height / from_angle(rotate_angle).sin();
    let trangle_for_rotation = TrangleForRotation {
        rotate_angle,
        height,
        width,
        max_line,
    };

    // get big angle data
    let height = radius;
    let left_angle = obtuse_angle / 2.;
    let top_angle = 90. - left_angle;
    let width = radius / from_angle(left_angle).tan();
    let big_trangle = BigTrangle {
        height,
        width,
        left_angle,
        top_angle,
    };

    // move
    let line_length = trangle_for_rotation.max_line - big_trangle.width;
    ctx.move_to(Z, Z);
    ctx.save().unwrap();
    ctx.rotate(rotate_angle);
    ctx.rel_line_to(line_length, Z);
    ctx.restore().unwrap();

    // rounded corner left
    let origin = (
        trangle_for_rotation.width + big_trangle.width,
        size.1 - big_trangle.height,
    );
    let angle_from_to = (
        from_angle(90. + 2. * big_trangle.top_angle),
        from_angle(90.),
    );
    ctx.arc(origin.0, origin.1, radius, angle_from_to.0, angle_from_to.1);

    // next straight line
    let origin_x = origin.0;
    let line_length = size.0 - 2. * origin_x;
    ctx.rel_line_to(line_length, Z);

    // rounded corner right
    let origin = (size.0 - origin.0, origin.1);
    let angle_from_to = (
        from_angle(90.),
        from_angle(90. - 2. * big_trangle.top_angle),
    );
    ctx.arc(origin.0, origin.1, radius, angle_from_to.0, angle_from_to.1);

    // final rotate line
    ctx.line_to(size.0, Z);

    if close_path {
        ctx.line_to(Z, Z);
        ctx.close_path();
    }

    let path = ctx.copy_path().unwrap();
    let text_left_start_x = trangle_for_rotation.width + big_trangle.width;

    Ok((path, text_left_start_x))
}

// top first. every thing is here
fn draw_common_top_or_right(conf: &DrawConfig, progress: f64) -> ImageSurface {
    let size = (
        (conf.size.0 - (conf.border_width * 2)) as f64,
        (conf.size.1 - conf.border_width) as f64,
    );
    let radius = conf.radius - conf.border_width as f64;
    let (path, text_left_start_x) = draw_slide_path(conf.obtuse_angle, radius, size, true).unwrap();

    let (surf, ctx) = conf.new_horizontal_surf();

    // bg also text x info
    let bg_size = (
        (conf.size.0 - conf.border_width * 2) as f64,
        (conf.size.1 - conf.border_width) as f64,
    );
    let radius = conf.radius;
    let (path, text_left_start_x) =
        draw_slide_path(conf.obtuse_angle, radius, bg_size, true).unwrap();
    ctx.save().unwrap();
    ctx.translate(conf.border_width as f64, Z);
    ctx.append_path(&path);
    ctx.set_source_color(&conf.bg_color);
    ctx.fill().unwrap();
    ctx.restore().unwrap();

    // fg
    let fg_size = ((bg_size.0 * progress).ceil(), bg_size.1);
    let (path, _) = draw_slide_path(conf.obtuse_angle, radius, fg_size, true).unwrap();
    let translate_x = match conf.progress_direction {
        Direction::Forward => conf.border_width as f64,
        Direction::Backward => conf.border_width as f64 + (bg_size.0 - fg_size.0),
    };
    ctx.save().unwrap();
    ctx.translate(translate_x, Z);
    ctx.append_path(&path);
    ctx.set_source_color(&conf.fg_color);
    ctx.fill().unwrap();
    ctx.restore().unwrap();

    // border
    let border_size = (
        (conf.size.0 - conf.border_width) as f64,
        (conf.size.1 - conf.border_width / 2) as f64,
    );
    let radius = conf.radius + conf.border_width as f64 / 2.;
    let (path, _) = draw_slide_path(conf.obtuse_angle, radius, border_size, false).unwrap();
    ctx.save().unwrap();
    ctx.translate(conf.border_width as f64 / 2., Z);
    ctx.append_path(&path);
    ctx.set_source_color(&conf.border_color);
    ctx.set_line_width(conf.border_width as f64);
    ctx.stroke().unwrap();
    ctx.restore().unwrap();

    // text

    surf
}

fn draw_top(conf: &DrawConfig, pressing: bool) -> ImageSurface {
    let (surf, ctx) = conf.new_horizontal_surf();

    let size = (conf.size.0 as f64, conf.size.1 as f64);
    let border_width = conf.border_width as f64;

    // path
    let radius = size.1 - border_width;
    ctx.arc(size.1, Z, radius, 0.5 * PI, 1. * PI);
    ctx.rel_line_to(size.0 - 2. * size.1, Z);
    ctx.arc(size.0 - size.1, Z, radius, 0. * PI, 0.5 * PI);
    ctx.rel_line_to(size.0 - 2. * border_width, Z);
    ctx.close_path();

    // content
    ctx.set_source_color(&conf.color);
    ctx.fill_preserve().unwrap();

    // mask
    let lg = if pressing {
        let lg = LinearGradient::new(size.0 / 2., Z, size.0 / 2., size.1);
        lg.add_color_stop_rgba(0., 0., 0., 0., 0.7);
        lg.add_color_stop_rgba(0.3, 0., 0., 0., 0.);
        lg.add_color_stop_rgba(1., 0., 0., 0., 0.7);
        lg
    } else {
        let lg = LinearGradient::new(size.0 / 2., Z, size.0 / 2., size.1);
        lg.add_color_stop_rgba(0., 0., 0., 0., 0.);
        lg.add_color_stop_rgba(0.4, 0., 0., 0., 0.);
        lg.add_color_stop_rgba(1., 0., 0., 0., 0.7);
        lg
    };
    ctx.set_source(&lg).unwrap();
    ctx.fill_preserve().unwrap();

    ctx.new_path();

    // border
    let radius = size.1 - (border_width / 2.);
    ctx.arc(size.1, Z, radius, 0.5 * PI, 1. * PI);
    ctx.rel_line_to(size.0 - 2. * size.1, Z);
    ctx.arc(size.0 - size.1, Z, radius, 0. * PI, 0.5 * PI);
    ctx.close_path();

    ctx.set_source_color(&conf.border_color);
    ctx.set_line_width(border_width);
    ctx.stroke_preserve().unwrap();

    surf
}

fn draw_right(conf: &DrawConfig, pressing: bool) -> ImageSurface {
    let (surf, ctx) = conf.new_vertical_surf();
    let base = draw_top(conf, pressing);

    ctx.rotate(90.0_f64.to_radians());
    ctx.set_source_surface(&base, 0., 0.).unwrap();
    ctx.paint().unwrap();

    surf
}

fn draw_bottom(conf: &DrawConfig, pressing: bool) -> ImageSurface {
    let (surf, ctx) = conf.new_horizontal_surf();
    let base = draw_top(conf, pressing);

    ctx.rotate(180.0_f64.to_radians());
    ctx.translate(-conf.size.0 as f64, -conf.size.1 as f64);
    ctx.set_source_surface(&base, 0., 0.).unwrap();
    ctx.paint().unwrap();

    surf
}

fn draw_left(conf: &DrawConfig, pressing: bool) -> ImageSurface {
    let (surf, ctx) = conf.new_vertical_surf();
    let base = draw_top(conf, pressing);

    ctx.rotate(-90.0_f64.to_radians());
    ctx.translate(-conf.size.0 as f64, Z);
    ctx.set_source_surface(&base, 0., 0.).unwrap();
    ctx.paint().unwrap();

    surf
}

pub(super) fn make_draw_func(btn_conf: &BtnConfig, edge: Edge) -> impl Fn(bool) -> ImageSurface {
    let draw_conf = DrawConfig::new(btn_conf);

    let func = match edge {
        Edge::Left => draw_left,
        Edge::Right => draw_right,
        Edge::Top => draw_top,
        Edge::Bottom => draw_bottom,
        _ => unreachable!(),
    };

    move |pressing: _| func(&draw_conf, pressing)
}
