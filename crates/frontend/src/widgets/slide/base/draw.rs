use config::widgets::slide::base::SlideConfig;
use gdk::prelude::GdkCairoContextExt;
use smithay_client_toolkit::shell::wlr_layer::Anchor;
use util::text::TextConfig;

use std::f64::consts::PI;

use cairo::{self, Context, ImageSurface, Path};
use gdk::RGBA;
use util::draw::new_surface;
use util::{rgba_to_color, Z};

#[derive(Debug)]
pub struct DrawConfig {
    length: i32,
    thickness: i32,
    border_width: i32,

    obtuse_angle: f64,
    radius: f64,

    bg_color: RGBA,
    pub fg_color: RGBA,
    border_color: RGBA,

    func: fn(&DrawConfig, f64) -> ImageSurface,
}
impl DrawConfig {
    pub fn new(slide_conf: &SlideConfig, edge: Anchor) -> Self {
        let content_size = slide_conf.size().unwrap();

        let func = match edge {
            Anchor::LEFT => draw_left,
            Anchor::RIGHT => draw_right,
            Anchor::TOP => draw_top,
            Anchor::BOTTOM => draw_bottom,
            _ => unreachable!(),
        };

        Self {
            length: content_size.1.ceil() as i32,
            thickness: content_size.0.ceil() as i32,
            border_width: slide_conf.border_width,
            obtuse_angle: slide_conf.obtuse_angle,
            radius: slide_conf.radius,
            bg_color: slide_conf.bg_color,
            fg_color: slide_conf.fg_color,
            border_color: slide_conf.border_color,
            func,
        }
    }
    fn new_horizontal_surf(&self) -> (ImageSurface, Context) {
        let surf = new_surface((self.length, self.thickness));
        let ctx = cairo::Context::new(&surf).unwrap();
        (surf, ctx)
    }
    fn new_vertical_surf(&self) -> (ImageSurface, Context) {
        let surf = new_surface((self.thickness, self.length));
        let ctx = cairo::Context::new(&surf).unwrap();
        (surf, ctx)
    }
    pub fn draw(&self, p: f64) -> ImageSurface {
        (self.func)(self, p)
    }
}

fn draw_text(progress: f64, progress_thickness: i32) -> ImageSurface {
    let height = (progress_thickness as f64 * 0.9).ceil() as i32;
    let text = format!("{:.2}%", progress * 100.);
    util::text::draw_text(
        &text,
        TextConfig::new(None, rgba_to_color(RGBA::BLACK), height),
    )
    .to_image_surface()

    // let pg_ctx = get_pango_context();
    // let mut desc = pg_ctx.font_description().unwrap();
    // desc.set_absolute_size(height * 1024.);
    // pg_ctx.set_font_description(Some(&desc));
    // let layout = pangocairo::pango::Layout::new(&pg_ctx);
    //
    // let text = format!("{:.2}%", progress * 100.);
    // layout.set_text(&text);
    //
    // let (_, logic) = layout.pixel_extents();
    // let scale = height / logic.height() as f64;
    //
    // let size = ((logic.width() as f64 * scale).ceil() as i32, height as i32);
    // let surf = new_surface(size);
    // let ctx = cairo::Context::new(&surf).unwrap();
    // ctx.scale(scale, scale);
    // pangocairo::functions::show_layout(&ctx, &layout);
    //
    // surf
}

fn draw_slide_path(
    obtuse_angle: f64,
    radius: f64,
    size: (f64, f64),
    close_path: bool,
) -> Result<Path, String> {
    let ctx =
        cairo::Context::new(new_surface((size.0.ceil() as i32, size.1.ceil() as i32))).unwrap();

    #[allow(dead_code)]
    struct BigTrangle {
        height: f64,
        width: f64,
        left_angle: f64,
        top_angle: f64,
    }

    #[allow(dead_code)]
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
    fg_surf: ImageSurface,
    bg_size: (f64, f64),
    normal_text_surf: ImageSurface,
}
impl DrawData {
    fn new_surface_bar(&self) -> (cairo::ImageSurface, cairo::Context) {
        let surf = new_surface((self.bar.width(), self.bar.height()));
        let ctx = cairo::Context::new(&surf).unwrap();
        (surf, ctx)
    }
    fn make_text(&self, conf: &DrawConfig) -> (ImageSurface, ImageSurface) {
        let normal_text_surf = &self.normal_text_surf;
        let text_start_pos = (
            ((self.bar.width() - normal_text_surf.width()) as f64 / 2.).floor(),
            ((self.bg_size.1 - normal_text_surf.height() as f64) / 2.).floor(),
        );

        let bg_text_surf = {
            let (bg_text_surf, ctx) = self.new_surface_bar();
            ctx.mask_surface(normal_text_surf, text_start_pos.0, text_start_pos.1)
                .unwrap();

            let (mask_surf, ctx) = self.new_surface_bar();
            ctx.set_source_color(&conf.bg_color);
            ctx.mask_surface(&self.fg_surf, Z, Z).unwrap();

            let (final_surf, ctx) = self.new_surface_bar();
            ctx.set_source_surface(mask_surf, Z, Z).unwrap();
            ctx.mask_surface(&bg_text_surf, Z, Z).unwrap();

            final_surf
        };

        let fg_text_surf = {
            let (fg_text_surf, ctx) = self.new_surface_bar();
            ctx.set_source_color(&conf.fg_color);
            ctx.mask_surface(normal_text_surf, text_start_pos.0, text_start_pos.1)
                .unwrap();

            // ctx.translate(conf.border_width as f64 + self.text_translate_x, Z);
            // ctx.append_path(&self.bg_path);
            // ctx.set_operator(cairo::Operator::Clear);
            // ctx.fill().unwrap();

            fg_text_surf
        };

        (bg_text_surf, fg_text_surf)
    }
    fn draw_text_on_ctx(&self, ctx: &cairo::Context, conf: &DrawConfig) {
        let (bg_text, fg_text) = self.make_text(conf);
        ctx.set_source_surface(&fg_text, Z, Z).unwrap();
        ctx.paint().unwrap();
        ctx.set_source_surface(&bg_text, Z, Z).unwrap();
        ctx.paint().unwrap();
    }
}

fn make_draw_data(conf: &DrawConfig, progress: f64, is_forward: bool) -> DrawData {
    let (surf, ctx) = conf.new_horizontal_surf();

    // bg
    let bg_size = (
        (conf.length - conf.border_width * 2) as f64,
        (conf.thickness - conf.border_width) as f64,
    );
    let radius = conf.radius;
    let bg_path = draw_slide_path(conf.obtuse_angle, radius, bg_size, true).unwrap();
    ctx.save().unwrap();
    ctx.translate(conf.border_width as f64, Z);
    ctx.append_path(&bg_path);
    ctx.set_source_color(&conf.bg_color);
    ctx.fill().unwrap();
    ctx.restore().unwrap();

    // fg
    let fg_size = ((bg_size.0 * progress).ceil(), bg_size.1);
    let translate_x = match is_forward {
        true => -(bg_size.0 - fg_size.0),
        false => bg_size.0 - fg_size.0,
    };
    let fg_surf = new_surface((surf.width(), surf.height()));
    let fg_ctx = Context::new(&fg_surf).unwrap();
    fg_ctx.translate(conf.border_width as f64 + translate_x, Z);
    fg_ctx.append_path(&bg_path);
    fg_ctx.set_source_color(&conf.fg_color);
    fg_ctx.fill().unwrap();
    ctx.save().unwrap();
    ctx.translate(conf.border_width as f64, Z);
    ctx.append_path(&bg_path);
    ctx.restore().unwrap();
    ctx.set_source_surface(&fg_surf, Z, Z).unwrap();
    ctx.fill().unwrap();

    // border
    let border_size = (
        (conf.length - conf.border_width) as f64,
        (conf.thickness as f64 - conf.border_width as f64 / 2.),
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

    // text
    let normal_text_surf = draw_text(progress, fg_size.1 as i32);

    DrawData {
        bar: surf,
        fg_surf,
        bg_size,
        normal_text_surf,
    }
}

fn draw_top(conf: &DrawConfig, progress: f64) -> ImageSurface {
    let (surf, ctx) = conf.new_horizontal_surf();
    let draw_data = make_draw_data(conf, progress, true);

    ctx.set_source_surface(&draw_data.bar, Z, Z).unwrap();
    ctx.paint().unwrap();

    draw_data.draw_text_on_ctx(&ctx, conf);

    surf
}

fn draw_left(conf: &DrawConfig, progress: f64) -> ImageSurface {
    let top = draw_top(conf, progress);
    let (surf, ctx) = conf.new_vertical_surf();

    ctx.rotate(-90.0_f64.to_radians());
    ctx.translate(-surf.height() as f64, Z);

    ctx.set_source_surface(top, Z, Z).unwrap();
    ctx.paint().unwrap();

    surf
}

fn draw_right(conf: &DrawConfig, progress: f64) -> ImageSurface {
    let (surf, ctx) = conf.new_vertical_surf();
    let draw_data = make_draw_data(conf, progress, false);

    ctx.rotate(90.0_f64.to_radians());
    ctx.translate(Z, -conf.thickness as f64);

    ctx.set_source_surface(&draw_data.bar, Z, Z).unwrap();
    ctx.paint().unwrap();
    draw_data.draw_text_on_ctx(&ctx, conf);

    surf
}

fn draw_bottom(conf: &DrawConfig, progress: f64) -> ImageSurface {
    let (surf, ctx) = conf.new_horizontal_surf();
    let mut draw_data = make_draw_data(conf, progress, false);

    ctx.save().unwrap();
    ctx.rotate(180.0_f64.to_radians());
    ctx.translate(-surf.width() as f64, -surf.height() as f64);
    ctx.set_source_surface(&draw_data.bar, Z, Z).unwrap();
    ctx.paint().unwrap();
    ctx.restore().unwrap();

    {
        let (surf, ctx) = draw_data.new_surface_bar();
        ctx.rotate(180.0_f64.to_radians());
        ctx.translate(-surf.width() as f64, -surf.height() as f64);
        ctx.translate(Z, conf.border_width as f64);
        // let translate_y =
        //     ((draw_data.bg_size.1 - draw_data.normal_text_surf.height() as f64) / 2.).floor();
        ctx.set_source_surface(&draw_data.fg_surf, Z, Z).unwrap();
        ctx.paint().unwrap();
        draw_data.fg_surf = surf;
    };

    ctx.translate(Z, conf.border_width as f64);
    draw_data.draw_text_on_ctx(&ctx, conf);

    surf
}
