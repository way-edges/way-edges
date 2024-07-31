use std::{cell::RefCell, rc::Rc};

use cairo::{Format, ImageSurface, Path};
use gtk::{gdk::RGBA, prelude::GdkCairoContextExt};

use crate::{
    config::widgets::wrapbox::OutlookWindowConfig,
    ui::{
        draws::{shape::draw_rect_path, util::Z},
        widgets::wrapbox::MousePosition,
    },
};

/// cache
#[derive(Debug)]
pub struct Cache {
    pub border_path: Path,
    pub border: ImageSurface,
    pub window_path: Path,
    pub window: ImageSurface,
    pub window_shadow: ImageSurface,
    pub content_box_size: (f64, f64),
    pub startoff_point: (f64, f64),
    pub size: (f64, f64),
    pub content_size: (f64, f64),
    pub margins: [f64; 4],
}

pub type BoxOutlookWindowRc = Rc<RefCell<BoxOutlookWindow>>;

#[derive(Debug)]
pub struct BoxOutlookWindow {
    pub cache: Option<Cache>,
    config: OutlookWindowConfig,
}

impl BoxOutlookWindow {
    pub fn redraw(&mut self, content_size: (f64, f64)) {
        let margins = self.config.margins;
        let color = self.config.color;
        let border_radius = self.config.border_radius;
        let border_width = self.config.border_width;

        let ([content_box_size, size, startoff_point], margins) =
            Self::calculate_info(content_size, margins, border_width);
        let box_color = {
            let mut shade = RGBA::BLACK;
            shade.set_alpha(0.2);
            let one = shade;
            let two = color;
            let a = 1. - (1. - one.alpha()) * (1. - two.alpha());
            let r = (one.red() * one.alpha() + two.red() * two.alpha() * (1. - one.alpha())) / a;
            let g =
                (one.green() * one.alpha() + two.green() * two.alpha() * (1. - one.alpha())) / a;
            let b = (one.blue() * one.alpha() + two.blue() * two.alpha() * (1. - one.alpha())) / a;
            RGBA::new(r, g, b, a)
        };

        let new_surface =
            move |s: (i32, i32)| ImageSurface::create(Format::ARgb32, s.0, s.1).unwrap();

        let (border_path, border) = {
            let path = draw_rect_path(border_radius, size, [false, true, true, false]).unwrap();
            let map_size = (size.0.ceil() as i32, size.1.ceil() as i32);
            let surf = new_surface(map_size);
            let ctx = cairo::Context::new(&surf).unwrap();
            ctx.set_source_color(&color);
            ctx.append_path(&path);
            ctx.fill().unwrap();
            (path, surf)
        };

        let (window_path, window, window_shadow) = {
            let map_size = (
                content_box_size.0.ceil() as i32,
                content_box_size.1.ceil() as i32,
            );
            let path =
                draw_rect_path(border_radius, content_box_size, [true, true, true, true]).unwrap();
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

                let shadow_size = 10.0_f64.min(content_box_size.0 * 0.3);
                g([Z, Z, shadow_size, Z], &RGBA::BLACK);
                g([Z, Z, Z, shadow_size], &RGBA::BLACK);
                g(
                    [Z, content_box_size.1, Z, content_box_size.1 - shadow_size],
                    &RGBA::BLACK,
                );
                surf
            };

            (path, bg_surf, shadow_surf)
        };
        let cache = Cache {
            window_path,
            window,
            window_shadow,
            border_path,
            border,
            content_box_size,
            startoff_point,
            size,
            content_size,
            margins,
        };
        self.cache = Some(cache);
    }
    /// margins: left, top, right, bottom
    pub fn new(config: OutlookWindowConfig) -> Self {
        Self {
            cache: None,
            config,
        }
    }

    pub fn calculate_info(
        content_size: (f64, f64),
        margins: Option<[f64; 4]>,
        border_width: f64,
    ) -> ([(f64, f64); 3], [f64; 4]) {
        let margins = margins.unwrap_or([0., 0., 0., 0.]);
        let content_box_size = (
            (content_size.0 + margins[0] + margins[2]),
            (content_size.1 + margins[1] + margins[3]),
        );
        let size = (
            (content_box_size.0) + border_width * 2.,
            (content_box_size.1) + border_width * 2.,
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
        let cache = self.cache.as_ref().unwrap();
        let surf = ImageSurface::create(
            Format::ARgb32,
            cache.size.0.ceil() as i32,
            cache.size.1.ceil() as i32,
        )
        .unwrap();
        let ctx = cairo::Context::new(&surf).unwrap();

        // border
        ctx.set_source_surface(&cache.border, Z, Z).unwrap();
        ctx.paint().unwrap();

        ctx.translate(cache.startoff_point.0, cache.startoff_point.1);

        // content background
        ctx.set_source_surface(&cache.window, Z, Z).unwrap();
        ctx.paint().unwrap();

        // content
        ctx.set_source_surface(&content, cache.margins[0], cache.margins[1])
            .unwrap();
        ctx.append_path(&cache.window_path);
        ctx.fill().unwrap();

        // shadow
        ctx.set_source_surface(&cache.window_shadow, Z, Z).unwrap();
        ctx.paint().unwrap();

        surf
    }

    pub fn transform_mouse_pos(&self, pos: MousePosition) -> MousePosition {
        let cache = self.cache.as_ref().unwrap();
        let sp = cache.startoff_point;
        (pos.0 - sp.0, pos.1 - sp.1)
    }
}
