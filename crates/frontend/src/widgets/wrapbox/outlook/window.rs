use cairo::ImageSurface;

use config::widgets::wrapbox::{OutlookMargins, OutlookWindowConfig};
use gdk::{prelude::GdkCairoContextExt, RGBA};
use smithay_client_toolkit::shell::wlr_layer::Anchor;
use util::{
    draw::{color_mix, draw_rect_path, new_surface},
    Z,
};
use way_edges_derive::wrap_rc;

#[wrap_rc(rc = "pub")]
#[derive(Debug)]
pub struct DrawConf {
    margins: OutlookMargins,
    color: RGBA,
    border_radius: i32,
    border_width: i32,

    corners: [bool; 4],
}
impl DrawConf {
    pub fn new(outlook: OutlookWindowConfig, edge: Anchor) -> Self {
        let corners = match edge {
            Anchor::LEFT => [false, true, true, false],
            Anchor::RIGHT => [true, false, false, true],
            Anchor::TOP => [false, false, true, true],
            Anchor::BOTTOM => [true, true, false, false],
            _ => unreachable!(),
        };

        Self {
            margins: outlook.margins.clone(),
            color: outlook.color,
            border_radius: outlook.border_radius,
            border_width: outlook.border_width,
            corners,
        }
    }
    pub fn draw(&mut self, content: ImageSurface) -> ImageSurface {
        draw_combine(self, content)
    }
    pub fn translate_mouse_position(&self, pos: (f64, f64)) -> (f64, f64) {
        // pos - border - margin
        (
            pos.0 - self.border_width as f64 - self.margins.left as f64,
            pos.1 - self.border_width as f64 - self.margins.top as f64,
        )
    }
}

struct DrawBase {
    bg: ImageSurface,
    border_with_shadow: ImageSurface,
}

fn draw_base(conf: &DrawConf, content_size: (i32, i32)) -> DrawBase {
    let border_radius = conf.border_radius as f64;
    let corners = conf.corners;

    // calculate_info for later use
    let content_box_size = (
        conf.margins.left + conf.margins.right + content_size.0,
        conf.margins.top + conf.margins.bottom + content_size.1,
    );
    let total_size = (
        content_box_size.0 + conf.border_width * 2,
        content_box_size.1 + conf.border_width * 2,
    );

    // make float var for later use
    let f_content_box_size = (content_box_size.0 as f64, content_box_size.1 as f64);
    let f_total_size = (total_size.0 as f64, total_size.1 as f64);

    // mix color of border color and shadow(black)
    let box_color = color_mix(RGBA::new(0., 0., 0., 0.2), conf.color);

    // bg
    let (bg_path, bg) = {
        let path = draw_rect_path(border_radius, f_total_size, corners).unwrap();
        let surf = new_surface(total_size);
        let ctx = cairo::Context::new(&surf).unwrap();
        ctx.set_source_color(&box_color);
        ctx.append_path(&path);
        ctx.fill().unwrap();
        (path, surf)
    };

    // shadow
    let shadow_surf = {
        fn inside_grandient(p: [f64; 4], color: [f64; 3]) -> cairo::LinearGradient {
            let [r, g, b] = color;

            let t = cairo::LinearGradient::new(p[0], p[1], p[2], p[3]);
            t.add_color_stop_rgba(0., r, g, b, 0.4);
            t.add_color_stop_rgba(0.3, r, g, b, 0.1);
            t.add_color_stop_rgba(1., r, g, b, 0.);
            t
        }

        let surf = new_surface(content_box_size);
        let ctx = cairo::Context::new(&surf).unwrap();
        let g = |p: [f64; 4], c: [f64; 3]| {
            let t = inside_grandient(p, c);
            ctx.set_source(t).unwrap();
            ctx.paint().unwrap();
        };

        let shadow_size = 10.0_f64.min(f_content_box_size.0 * 0.3);
        let color = {
            let color = RGBA::BLACK;
            [
                color.red() as f64,
                color.green() as f64,
                color.blue() as f64,
            ]
        };
        // left, top, right, bottom
        g([Z, Z, shadow_size, Z], color);
        g([Z, Z, Z, shadow_size], color);
        g(
            [
                f_content_box_size.0,
                Z,
                f_content_box_size.0 - shadow_size,
                Z,
            ],
            color,
        );
        g(
            [
                Z,
                f_content_box_size.1,
                Z,
                f_content_box_size.1 - shadow_size,
            ],
            color,
        );
        surf
    };

    // border
    let border_surf = {
        let surf = new_surface(total_size);
        let ctx = cairo::Context::new(&surf).unwrap();
        ctx.set_source_color(&conf.color);
        ctx.append_path(&bg_path);
        ctx.fill().unwrap();

        ctx.set_operator(cairo::Operator::Clear);
        let path =
            draw_rect_path(border_radius, f_content_box_size, [true, true, true, true]).unwrap();
        ctx.translate(conf.border_width as f64, conf.border_width as f64);
        ctx.append_path(&path);
        ctx.fill().unwrap();

        surf
    };

    // combine border and shadow
    let border_with_shadow = {
        let surf = new_surface(total_size);
        let ctx = cairo::Context::new(&surf).unwrap();
        ctx.save().unwrap();
        ctx.translate(conf.border_width as f64, conf.border_width as f64);
        ctx.set_source_surface(shadow_surf, Z, Z).unwrap();
        ctx.paint().unwrap();
        ctx.restore().unwrap();

        ctx.set_source_surface(border_surf, Z, Z).unwrap();
        ctx.paint().unwrap();

        surf
    };

    DrawBase {
        bg,
        border_with_shadow,
    }
}

fn draw_combine(conf: &DrawConf, content: ImageSurface) -> ImageSurface {
    let base = draw_base(conf, (content.width(), content.height()));

    let surf = new_surface((base.bg.width(), base.bg.height()));
    let ctx = cairo::Context::new(&surf).unwrap();

    ctx.set_source_surface(base.bg, Z, Z).unwrap();
    ctx.paint().unwrap();

    ctx.save().unwrap();
    ctx.translate(
        (conf.border_width + conf.margins.left) as f64,
        (conf.border_width + conf.margins.top) as f64,
    );
    ctx.set_source_surface(&content, Z, Z).unwrap();
    ctx.paint().unwrap();
    ctx.restore().unwrap();

    ctx.set_source_surface(base.border_with_shadow, Z, Z)
        .unwrap();
    ctx.paint().unwrap();

    surf
}
