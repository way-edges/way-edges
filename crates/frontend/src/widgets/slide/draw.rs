use cairo::Path;
use config::widgets::slide::base::SlideConfig;
use gtk::cairo::Context;
use gtk::prelude::*;
use gtk4_layer_shell::Edge;

use std::cell::RefCell;
use std::f64::consts::PI;
use std::ops::Deref;
use std::rc::Rc;

use gtk::cairo::{self, ImageSurface};
use gtk::gdk::RGBA;
use util::draw::new_surface;
use util::Z;

use super::font::get_pango_context;

pub(super) struct DrawConfig {
    size: (i32, i32),
    border_width: i32,

    obtuse_angle: f64,
    radius: f64,

    bg_color: RGBA,
    fg_color: RGBA,
    border_color: RGBA,
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

fn draw_text(progress: f64, progress_thickness: i32) -> ImageSurface {
    let height = (progress_thickness as f64 * 0.9).ceil();
    let pg_ctx = get_pango_context();
    let mut desc = pg_ctx.font_description().unwrap();
    desc.set_absolute_size(height * 1024.);
    pg_ctx.set_font_description(Some(&desc));
    let layout = pangocairo::pango::Layout::new(&pg_ctx);

    let text = format!("{:.2}%", progress * 100.);
    layout.set_text(&text);

    let (_, logic) = layout.pixel_extents();
    let scale = height / logic.height() as f64;

    let size = ((logic.width() as f64 * scale).ceil() as i32, height as i32);
    let surf = new_surface(size);
    let ctx = cairo::Context::new(&surf).unwrap();
    ctx.scale(scale, scale);
    pangocairo::functions::show_layout(&ctx, &layout);

    surf
}

fn draw_slide_path(
    obtuse_angle: f64,
    radius: f64,
    size: (f64, f64),
    close_path: bool,
) -> Result<Path, String> {
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
    let percentage =
        (trangle_for_rotation.max_line - big_trangle.width) / trangle_for_rotation.max_line;
    ctx.move_to(Z, Z);
    ctx.rel_line_to(
        trangle_for_rotation.width * percentage,
        trangle_for_rotation.height * percentage,
    );

    // rounded corner left
    let origin = (
        trangle_for_rotation.width + big_trangle.width,
        size.1 - big_trangle.height,
    );
    let angle_from_to = (
        from_angle(90. + 2. * big_trangle.top_angle),
        from_angle(90.),
    );
    ctx.arc_negative(origin.0, origin.1, radius, angle_from_to.0, angle_from_to.1);

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
    ctx.arc_negative(origin.0, origin.1, radius, angle_from_to.0, angle_from_to.1);

    // final rotate line
    ctx.line_to(size.0, Z);

    if close_path {
        ctx.line_to(Z, Z);
        ctx.close_path();
    }

    let path = ctx.copy_path().unwrap();

    Ok(path)
}

struct DrawData {
    bar: ImageSurface,
    fg_path: cairo::Path,
    fg_size: (f64, f64),
    bg_size: (f64, f64),
    bg_text: ImageSurface,
    fg_text: ImageSurface,
}

fn make_draw_data(conf: &DrawConfig, progress: f64, is_forward: bool) -> DrawData {
    let (surf, ctx) = conf.new_horizontal_surf();

    // bg
    let bg_size = (
        (conf.size.0 - conf.border_width * 2) as f64,
        (conf.size.1 - conf.border_width) as f64,
    );
    let radius = conf.radius;
    let path = draw_slide_path(conf.obtuse_angle, radius, bg_size, true).unwrap();
    ctx.save().unwrap();
    ctx.translate(conf.border_width as f64, Z);
    ctx.append_path(&path);
    ctx.set_source_color(&conf.bg_color);
    ctx.fill().unwrap();
    ctx.restore().unwrap();

    // fg
    let fg_size = ((bg_size.0 * progress).ceil(), bg_size.1);
    let fg_surf = {
        let translate_x = match is_forward {
            true => -(bg_size.0 - fg_size.0),
            false => bg_size.0 - fg_size.0,
        };
        let fg_surf = new_surface((surf.width(), surf.height()));
        let fg_ctx = Context::new(&fg_surf).unwrap();
        fg_ctx.translate(conf.border_width as f64 + translate_x, Z);
        fg_ctx.append_path(&path);
        fg_ctx.set_source_color(&conf.fg_color);
        fg_ctx.fill().unwrap();
        fg_surf
    };
    ctx.save().unwrap();
    ctx.append_path(&path);
    ctx.set_source_surface(&fg_surf, Z, Z).unwrap();
    ctx.fill().unwrap();
    ctx.restore().unwrap();

    // border
    let border_size = (
        (conf.size.0 - conf.border_width) as f64,
        (conf.size.1 as f64 - conf.border_width as f64 / 2.),
    );
    let radius = conf.radius + conf.border_width as f64 / 2.;
    let path = draw_slide_path(conf.obtuse_angle, radius, border_size, false).unwrap();
    ctx.save().unwrap();
    ctx.translate(conf.border_width as f64 / 2., Z);
    ctx.append_path(&path);
    ctx.set_source_color(&conf.border_color);
    ctx.set_line_width(conf.border_width as f64);
    ctx.stroke().unwrap();
    ctx.restore().unwrap();

    let normal_text_surf = draw_text(progress, fg_size.1 as i32);
    let text_start_pos = (
        ((surf.width() - normal_text_surf.width()) as f64 / 2.).floor(),
        ((bg_size.1 - normal_text_surf.height() as f64) / 2.).floor(),
    );
    let bg_text_surf = {
        let bg_text_surf = new_surface((surf.width(), surf.height()));
        let ctx = cairo::Context::new(&bg_text_surf).unwrap();
        ctx.mask_surface(&normal_text_surf, text_start_pos.0, text_start_pos.1)
            .unwrap();

        let mask_surf = new_surface((fg_surf.width(), fg_surf.height()));
        let ctx = cairo::Context::new(&mask_surf).unwrap();
        ctx.set_source_color(&conf.bg_color);
        ctx.mask_surface(&fg_surf, Z, Z).unwrap();

        let final_surf = new_surface((surf.width(), surf.height()));
        let ctx = cairo::Context::new(&final_surf).unwrap();
        ctx.set_source_surface(mask_surf, Z, Z).unwrap();
        ctx.mask_surface(&bg_text_surf, Z, Z).unwrap();

        final_surf
    };
    let fg_text_surf = {
        let fg_text_surf = new_surface((surf.width(), surf.height()));
        let ctx = cairo::Context::new(&fg_text_surf).unwrap();
        ctx.set_source_color(&conf.fg_color);
        ctx.mask_surface(&normal_text_surf, text_start_pos.0, text_start_pos.1)
            .unwrap();
        ctx.clip();
        ctx.set_source_surface(&fg_surf, Z, Z).unwrap();
        ctx.fill().unwrap();
        fg_text_surf
    };

    DrawData {
        bar: surf,
        fg_path: path,
        fg_size,
        bg_size,
        bg_text: bg_text_surf,
        fg_text: fg_text_surf,
    }
}

fn draw_top(conf: &DrawConfig, progress: f64) -> ImageSurface {
    let (surf, ctx) = conf.new_horizontal_surf();
    let draw_data = make_draw_data(conf, progress, true);

    ctx.set_source_surface(draw_data.bar, Z, Z).unwrap();
    ctx.paint().unwrap();

    // text
    ctx.set_source_surface(&draw_data.fg_text, Z, Z).unwrap();
    ctx.paint().unwrap();
    ctx.set_source_surface(&draw_data.bg_text, Z, Z).unwrap();
    ctx.paint().unwrap();

    surf
}

fn draw_left(conf: &DrawConfig, progress: f64) -> ImageSurface {
    let top = draw_top(conf, progress);
    let (surf, ctx) = conf.new_vertical_surf();

    ctx.rotate(-90.0_f64.to_radians());
    ctx.translate(-surf.width() as f64, Z);

    ctx.set_source_surface(top, Z, Z).unwrap();
    ctx.paint().unwrap();

    surf
}

fn draw_right(conf: &DrawConfig, progress: f64) -> ImageSurface {
    let (surf, ctx) = conf.new_horizontal_surf();
    let draw_data = make_draw_data(conf, progress, false);

    ctx.rotate(90.0_f64.to_radians());

    ctx.set_source_surface(draw_data.bar, Z, Z).unwrap();
    ctx.paint().unwrap();

    // text
    ctx.set_source_surface(&draw_data.fg_text, Z, Z).unwrap();
    ctx.paint().unwrap();
    ctx.set_source_surface(&draw_data.bg_text, Z, Z).unwrap();
    ctx.paint().unwrap();

    surf
}

fn draw_bottom(conf: &DrawConfig, progress: f64) -> ImageSurface {
    let (surf, ctx) = conf.new_horizontal_surf();
    let draw_data = make_draw_data(conf, progress, true);

    ctx.rotate(-90.0_f64.to_radians());
    ctx.translate(-surf.width() as f64, Z);
    ctx.set_source_surface(draw_data.bar, Z, Z).unwrap();
    ctx.paint().unwrap();

    // text
    ctx.set_source_surface(&draw_data.fg_text, Z, Z).unwrap();
    ctx.paint().unwrap();
    ctx.set_source_surface(&draw_data.bg_text, Z, Z).unwrap();
    ctx.paint().unwrap();

    surf
}

pub(super) fn make_draw_func(
    slide_config: &SlideConfig,
    edge: Edge,
) -> (Rc<RefCell<DrawConfig>>, impl Fn(f64) -> ImageSurface) {
    let draw_conf = Rc::new(RefCell::new(DrawConfig::new(slide_config)));

    let func = match edge {
        Edge::Left => draw_left,
        Edge::Right => draw_right,
        Edge::Top => draw_top,
        Edge::Bottom => draw_bottom,
        _ => unreachable!(),
    };

    (draw_conf.clone(), move |progress| {
        func(draw_conf.borrow().deref(), progress)
    })
}
