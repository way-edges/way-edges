use cairo::{Format, ImageSurface, Path};
use gtk::{gdk::RGBA, prelude::GdkCairoContextExt};
use gtk4_layer_shell::Edge;

use crate::{
    config::widgets::wrapbox::OutlookWindowConfig,
    ui::{
        draws::{
            shape::draw_rect_path,
            util::{color_mix, new_surface, Z},
        },
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

    pub content_box_size: (i32, i32),
    pub border_startoff_point: (i32, i32),
    pub margin_startoff_point: (i32, i32),
    pub size: (i32, i32),
    pub content_size: (i32, i32),
    pub margins: [i32; 4],
}

#[derive(Debug)]
pub struct BoxOutlookWindow {
    pub cache: Cache,
    config: OutlookWindowConfig,
    edge: Edge,
}

impl BoxOutlookWindow {
    /// margins: left, top, right, bottom
    pub fn new(config: OutlookWindowConfig, initial_content_size: (i32, i32), edge: Edge) -> Self {
        Self {
            cache: Self::_redraw(&config, initial_content_size, edge),
            config,
            edge,
        }
    }

    fn _redraw(config: &OutlookWindowConfig, content_size: (i32, i32), edge: Edge) -> Cache {
        let margins = config.margins;
        let color = config.color;
        let border_radius = config.border_radius;
        let border_width = config.border_width;

        // calculate_info for later use
        let (
            [content_box_size, total_size],
            [border_startoff_point, margin_startoff_point],
            margins,
        ) = Self::calculate_info(content_size, margins, border_width, edge);

        // make float var for later use
        let f_content_box_size = (content_box_size.0 as f64, content_box_size.1 as f64);
        let f_total_size = (total_size.0 as f64, total_size.1 as f64);

        // mix color of border color and shadow(black)
        let box_color = {
            let mut shade = RGBA::BLACK;
            shade.set_alpha(0.2);
            let one = shade;
            let two = color;
            color_mix(one, two)
        };

        let new_surface =
            move |s: (i32, i32)| ImageSurface::create(Format::ARgb32, s.0, s.1).unwrap();

        // draw border (just a big rect)
        let (border_path, border) = {
            let path =
                draw_rect_path(border_radius, f_total_size, [false, true, true, false]).unwrap();
            let map_size = (total_size.0, total_size.1);
            let surf = new_surface(map_size);
            let ctx = cairo::Context::new(&surf).unwrap();
            ctx.set_source_color(&color);
            ctx.append_path(&path);
            ctx.fill().unwrap();
            (path, surf)
        };

        // window (inner rect), shadow only contains three direction
        let (window_path, window, window_shadow) = {
            let path = draw_rect_path(border_radius, f_content_box_size, [true, true, true, true])
                .unwrap();

            let map_size = (content_box_size.0, content_box_size.1);
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

                let shadow_size = 10.0_f64.min(f_content_box_size.0 * 0.3);
                g([Z, Z, shadow_size, Z], &RGBA::BLACK);
                g([Z, Z, Z, shadow_size], &RGBA::BLACK);
                g(
                    [
                        Z,
                        f_content_box_size.1,
                        Z,
                        f_content_box_size.1 - shadow_size,
                    ],
                    &RGBA::BLACK,
                );
                surf
            };

            (path, bg_surf, shadow_surf)
        };

        Cache {
            window_path,
            window,
            window_shadow,
            border_path,
            border,
            content_box_size,
            border_startoff_point,
            margin_startoff_point,
            size: total_size,
            content_size,
            margins,
        }
    }

    pub fn redraw_if_size_change(&mut self, content_size: (i32, i32)) {
        let size = self.cache.content_size;
        if size != content_size {
            self.cache = Self::_redraw(&self.config, content_size, self.edge);
        }
    }

    /// [ content_box_size, size ] with border, [ border_startoff_point, margin_startoff_point ], margins
    /// These all based on `edge` and rotate forward start from `left`
    pub fn calculate_info(
        content_size: (i32, i32),
        margins: Option<[i32; 4]>,
        border_width: i32,
        edge: Edge,
    ) -> ([(i32, i32); 2], [(i32, i32); 2], [i32; 4]) {
        let margins = margins.unwrap_or([0, 0, 0, 0]);
        let addition_margin = match edge {
            Edge::Left | Edge::Right => (margins[0] + margins[2], margins[1] + margins[3]),
            Edge::Top | Edge::Bottom => (margins[1] + margins[3], margins[0] + margins[2]),
            _ => unreachable!(),
        };
        let content_box_size = (
            (content_size.0 + addition_margin.0),
            (content_size.1 + addition_margin.1),
        );
        let size = (
            (content_box_size.0) + border_width * 2,
            (content_box_size.1) + border_width * 2,
        );
        let border_startoff_point = (
            ((size.0 as f64 - content_box_size.0 as f64) / 2.) as i32,
            ((size.1 as f64 - content_box_size.1 as f64) / 2.) as i32,
        );
        let margin_startoff_point = match edge {
            Edge::Left => (margins[0], margins[1]),
            Edge::Top => (margins[1], margins[2]),
            Edge::Right => (margins[2], margins[3]),
            Edge::Bottom => (margins[3], margins[0]),
            _ => unreachable!(),
        };
        println!("startoff_point: {:?}", border_startoff_point);
        (
            [content_box_size, size],
            [border_startoff_point, margin_startoff_point],
            margins,
        )
    }

    // draw box given content
    pub fn with_box(&self, content: ImageSurface) -> ImageSurface {
        let cache = &self.cache;
        let surf = new_surface(cache.size);
        let ctx = cairo::Context::new(&surf).unwrap();

        // border
        ctx.set_source_surface(&cache.border, Z, Z).unwrap();
        ctx.paint().unwrap();
        ctx.translate(
            cache.border_startoff_point.0 as f64,
            cache.border_startoff_point.1 as f64,
        );

        // content background
        ctx.set_source_surface(&cache.window, Z, Z).unwrap();
        ctx.paint().unwrap();

        // content
        ctx.set_source_surface(
            &content,
            cache.margin_startoff_point.0 as f64,
            cache.margin_startoff_point.1 as f64,
        )
        .unwrap();
        ctx.append_path(&cache.window_path);
        ctx.fill().unwrap();

        // shadow
        ctx.set_source_surface(&cache.window_shadow, Z, Z).unwrap();
        ctx.paint().unwrap();

        surf
    }

    pub fn transform_mouse_pos(&self, pos: MousePosition) -> MousePosition {
        let cache = &self.cache;
        let sp = cache.border_startoff_point;

        // pos - border - margin
        (
            pos.0 - sp.0 as f64 - self.cache.margins[0] as f64,
            pos.1 - sp.1 as f64 - self.cache.margins[1] as f64,
        )
    }
}
