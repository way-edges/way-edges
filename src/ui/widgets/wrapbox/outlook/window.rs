use cairo::{Format, ImageSurface, Path};
use gtk::{gdk::RGBA, prelude::GdkCairoContextExt};

use crate::ui::draws::{shape::draw_rect_path, util::Z};

/// cache
#[derive(Debug)]
pub struct BoxDrawsCache {
    pub border_path: Path,
    pub border: ImageSurface,
    pub window_path: Path,
    pub window: ImageSurface,
    pub window_shadow: ImageSurface,

    pub content_size: (f64, f64),
    pub size: (f64, f64),
    pub margins: [f64; 4],

    pub content_box_size: (f64, f64),
    pub startoff_point: (f64, f64),
}

impl BoxDrawsCache {
    /// margins: left, top, right, bottom
    pub fn new(
        content_size: (f64, f64),
        margins: Option<[f64; 4]>,
        border_color: RGBA,
        box_color: Option<RGBA>,
        radius_percentage: f64,
        size_factors: (f64, f64),
    ) -> Self {
        let ([content_box_size, size, startoff_point], margins) =
            Self::calculate_info(content_size, margins, size_factors);
        // let content_box_size = (
        //     (content_size.0 + margins[0] + margins[2]),
        //     (content_size.1 + margins[1] + margins[3]),
        // );
        // let size = ((content_box_size.0) / 0.75, (content_box_size.1) / 0.85);
        let box_color = box_color.unwrap_or_else(|| {
            let mut shade = RGBA::BLACK;
            shade.set_alpha(0.2);
            let one = shade;
            let two = border_color;
            let a = 1. - (1. - one.alpha()) * (1. - two.alpha());
            let r = (one.red() * one.alpha() + two.red() * two.alpha() * (1. - one.alpha())) / a;
            let g =
                (one.green() * one.alpha() + two.green() * two.alpha() * (1. - one.alpha())) / a;
            let b = (one.blue() * one.alpha() + two.blue() * two.alpha() * (1. - one.alpha())) / a;
            RGBA::new(r, g, b, a)
        });
        println!(
            "bg color: {:?}, border color: {:?}",
            box_color, border_color
        );

        let new_surface =
            move |s: (i32, i32)| ImageSurface::create(Format::ARgb32, s.0, s.1).unwrap();

        let (border_path, border) = {
            let path = draw_rect_path(size.0 * radius_percentage, size, [false, true, true, false])
                .unwrap();
            let map_size = (size.0.ceil() as i32, size.1.ceil() as i32);
            let surf = new_surface(map_size);
            let ctx = cairo::Context::new(&surf).unwrap();
            ctx.set_source_color(&border_color);
            ctx.append_path(&path);
            ctx.fill().unwrap();
            (path, surf)
        };

        let (window_path, window, window_shadow) = {
            let map_size = (
                content_box_size.0.ceil() as i32,
                content_box_size.1.ceil() as i32,
            );
            let path = draw_rect_path(
                content_box_size.0 * radius_percentage,
                content_box_size,
                [true, true, true, true],
            )
            .unwrap();
            let bg_surf = {
                let surf = new_surface(map_size);
                let ctx = cairo::Context::new(&surf).unwrap();
                ctx.set_source_color(&box_color);
                ctx.append_path(&path);
                ctx.fill().unwrap();
                surf
            };
            let shadow_surf = {
                fn inside_grandient(p: [f64; 4], c: &RGBA) -> cairo::LinearGradient {
                    let t = cairo::LinearGradient::new(p[0], p[1], p[2], p[3]);
                    t.add_color_stop_rgba(
                        0.,
                        c.red().into(),
                        c.green().into(),
                        c.blue().into(),
                        0.4,
                    );
                    t.add_color_stop_rgba(
                        0.3,
                        c.red().into(),
                        c.green().into(),
                        c.blue().into(),
                        0.1,
                    );
                    t.add_color_stop_rgba(
                        1.,
                        c.red().into(),
                        c.green().into(),
                        c.blue().into(),
                        0.,
                    );
                    t
                }

                let surf = new_surface(map_size);
                let ctx = cairo::Context::new(&surf).unwrap();
                let g = |p: [f64; 4], c: &RGBA| {
                    let t = inside_grandient(p, c);
                    ctx.set_source(t).unwrap();
                    ctx.append_path(&path);
                    ctx.fill().unwrap();
                };

                g([Z, Z, content_box_size.0 * 0.3, Z], &RGBA::BLACK);
                g([Z, Z, Z, content_box_size.0 * 0.3], &RGBA::BLACK);
                g(
                    [
                        Z,
                        content_box_size.1,
                        Z,
                        content_box_size.1 - content_box_size.0 * 0.3,
                    ],
                    &RGBA::BLACK,
                );
                surf
            };

            (path, bg_surf, shadow_surf)
        };
        BoxDrawsCache {
            border_path,
            border,
            window_path,
            window,
            window_shadow,
            startoff_point,
            content_size,
            content_box_size,
            size,
            margins,
        }
    }

    pub fn calculate_info(
        content_size: (f64, f64),
        margins: Option<[f64; 4]>,
        size_factors: (f64, f64),
    ) -> ([(f64, f64); 3], [f64; 4]) {
        let margins = margins.unwrap_or([0., 0., 0., 0.]);
        let content_box_size = (
            (content_size.0 + margins[0] + margins[2]),
            (content_size.1 + margins[1] + margins[3]),
        );
        let size = (
            (content_box_size.0) / size_factors.0,
            (content_box_size.1) / size_factors.1,
        );
        let startoff_point = (
            (size.0 - content_box_size.0) / 2.,
            (size.1 - content_box_size.1) / 2.,
        );
        ([content_box_size, size, startoff_point], margins)
    }

    /// get context_size
    pub fn calculate_info_reverse(
        map_size: (f64, f64),
        margins: Option<[f64; 4]>,
        size_factors: (f64, f64),
    ) -> (f64, f64) {
        let margins = margins.unwrap_or([0., 0., 0., 0.]);
        (
            (map_size.0 - margins[0] - margins[2]) * size_factors.0,
            (map_size.1 - margins[1] - margins[3]) * size_factors.1,
        )
    }

    pub fn with_box(&self, content: ImageSurface) -> ImageSurface {
        println!("self: {self:?}");
        let surf = ImageSurface::create(
            Format::ARgb32,
            self.size.0.ceil() as i32,
            self.size.1.ceil() as i32,
        )
        .unwrap();
        let ctx = cairo::Context::new(&surf).unwrap();

        // border
        ctx.set_source_surface(&self.border, Z, Z).unwrap();
        ctx.rectangle(Z, Z, self.size.0, self.size.1);
        ctx.fill().unwrap();

        ctx.translate(self.startoff_point.0, self.startoff_point.1);

        // content background
        ctx.set_source_surface(&self.window, Z, Z).unwrap();
        ctx.rectangle(Z, Z, self.content_box_size.0, self.content_box_size.1);
        ctx.fill().unwrap();

        // content
        ctx.set_source_surface(&content, self.margins[0], self.margins[1])
            .unwrap();
        ctx.append_path(&self.window_path);
        ctx.fill().unwrap();

        // shadow
        ctx.set_source_surface(&self.window_shadow, Z, Z).unwrap();
        ctx.rectangle(Z, Z, self.content_box_size.0, self.content_box_size.1);
        ctx.fill().unwrap();

        surf
    }
}
