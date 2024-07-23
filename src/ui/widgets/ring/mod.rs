/// NOTE: This widget can not be used directly
use std::{cell::RefCell, rc::Rc, time::Duration};

use cairo::{Format, ImageSurface};
use gtk::{gdk::RGBA, prelude::GdkCairoContextExt, DrawingArea};
use gtk::{glib, pango};
use interval_task::runner::{ExternalRunnerExt, Runner, Task};

use crate::ui::draws::{shape::draw_fan, util::Z};

use super::wrapbox::display::grid::DisplayWidget;
use super::wrapbox::BoxExposeRc;

#[derive(Debug)]
pub struct Ring {
    pub progress: f64,

    pub radius: f64,
    pub fg_color: RGBA,

    pub bg_arc: ImageSurface,
    pub inner_radius: f64,

    pub ring_surf: ImageSurface,
    pub text_surf: ImageSurface,
}
impl Ring {
    pub fn new(ring_width: f64, radius: f64, bg_color: RGBA, fg_color: RGBA) -> Self {
        let progress = 0.5;
        let (bg_arc, inner_radius) = Self::draw_base(radius, ring_width, &bg_color);
        let (ring_surf, text_surf) =
            Self::draw_progress(&bg_arc, inner_radius, &fg_color, progress, radius);

        Self {
            progress,
            radius,
            fg_color,
            bg_arc,
            inner_radius,
            ring_surf,
            text_surf,
        }
    }
    pub fn update_progress(&mut self, p: f64) {
        if p != self.progress {
            self.progress = p;
            self.redraw()
        }
    }
    fn redraw(&mut self) {
        (self.ring_surf, self.text_surf) = Self::draw_progress(
            &self.bg_arc,
            self.inner_radius,
            &self.fg_color,
            self.progress,
            self.radius,
        );
    }
    pub fn draw_progress(
        bg_arc: &ImageSurface,
        inner_radius: f64,
        fg_color: &RGBA,
        progress: f64,
        radius: f64,
    ) -> (ImageSurface, ImageSurface) {
        let size = (bg_arc.width(), bg_arc.height());
        let ring_surf = {
            let surf = ImageSurface::create(Format::ARgb32, size.0, size.1).unwrap();
            let ctx = cairo::Context::new(&surf).unwrap();

            ctx.set_source_surface(bg_arc, Z, Z).unwrap();
            ctx.paint().unwrap();

            ctx.set_source_color(fg_color);
            draw_fan(&ctx, (radius, radius), radius, 0., progress * 2.);
            ctx.fill().unwrap();

            ctx.set_operator(cairo::Operator::Clear);
            draw_fan(&ctx, (radius, radius), inner_radius, 0., 2.);
            ctx.fill().unwrap();
            surf
        };

        let text_surf = {
            let pc = pangocairo::pango::Context::new();
            let fm = pangocairo::FontMap::default();
            pc.set_font_map(Some(&fm));
            let mut desc = pc.font_description().unwrap();
            desc.set_absolute_size(radius * 1.5 * 1024.);
            desc.set_family("JetBrainsMono Nerd Font Mono");
            pc.set_font_description(Some(&desc));
            let pl = pangocairo::pango::Layout::new(&pc);

            pl.set_text(format!("Progress: {:.0}%", progress * 100.).as_str());
            println!("size: {:?}", pl.size());
            let size = pl.pixel_size();
            println!("pixel size: {:?}", size);

            let surf = ImageSurface::create(Format::ARgb32, size.0, size.1).unwrap();
            let ctx = cairo::Context::new(&surf).unwrap();
            ctx.set_antialias(cairo::Antialias::None);

            ctx.set_source_color(fg_color);
            pangocairo::functions::show_layout(&ctx, &pl);

            surf
        };

        (ring_surf, text_surf)
    }
    pub fn draw_base(radius: f64, ring_width: f64, bg_color: &RGBA) -> (ImageSurface, f64) {
        let big_radius = radius;
        let small_radius = big_radius - ring_width;
        let b_wh = (big_radius * 2.).ceil() as i32;

        let bg_surf = {
            let surf = ImageSurface::create(Format::ARgb32, b_wh, b_wh).unwrap();
            let ctx = cairo::Context::new(&surf).unwrap();

            ctx.set_source_color(bg_color);
            draw_fan(&ctx, (big_radius, big_radius), big_radius, 0., 2.);
            ctx.fill().unwrap();
            surf
        };

        (bg_surf, small_radius)
    }
}

struct RingEvents {
    queue_draw: Box<dyn FnMut()>,
}

pub struct RingCtx {
    cache_content: ImageSurface,
    inner: Rc<RefCell<Ring>>,
    runner: Option<Runner<Task>>,
}
impl RingCtx {
    pub fn new(
        events: RingEvents,
        inner: Rc<RefCell<Ring>>,
        update_interval: Duration,
        mut update_func: Box<dyn Send + FnMut() -> f64>,
    ) -> Self {
        let runner = {
            let mut runner = interval_task::runner::new_external_close_runner(update_interval);
            let (s, r) = async_channel::bounded(1);
            runner.set_task(Box::new(move || {
                let res = update_func();
                s.send_blocking(res);
            }));
            let mut queue_draw = events.queue_draw;
            glib::spawn_future_local(glib::clone!(
                // #[weak]
                // darea,
                #[weak]
                inner,
                async move {
                    while let Ok(res) = r.recv().await {
                        inner.borrow_mut().update_progress(res);
                        queue_draw();
                        // darea.queue_draw();
                    }
                    log::warn!("ring update runner closed");
                }
            ));
            runner.start();
            Some(runner)
        };
        let cache_content = {
            let a = inner.borrow();
            Self::_combine(&a.ring_surf, &a.text_surf)
        };
        Self {
            inner,
            runner,
            cache_content,
        }
    }

    fn combine(&mut self) {
        self.cache_content = {
            let a = self.inner.borrow();
            Self::_combine(&a.ring_surf, &a.text_surf)
        };
    }
    fn _combine(r: &ImageSurface, t: &ImageSurface) -> ImageSurface {
        println!(
            "{}, {}, {}, {}",
            r.width(),
            t.width(),
            r.height(),
            t.height(),
        );
        let size = (r.width() + t.width(), r.height().max(t.height()));
        let surf = ImageSurface::create(Format::ARgb32, size.0, size.1).unwrap();
        let ctx = cairo::Context::new(&surf).unwrap();
        ctx.set_antialias(cairo::Antialias::None);
        ctx.set_source_surface(r, Z, Z).unwrap();
        ctx.paint().unwrap();
        ctx.set_source_surface(t, r.width() as f64, Z).unwrap();
        ctx.paint().unwrap();

        surf
    }
}

impl DisplayWidget for RingCtx {
    fn get_size(&mut self) -> (f64, f64) {
        let c = &self.cache_content;
        println!("get_width: {:?}", c.width());
        (c.width() as f64, c.height() as f64)
    }

    fn content(&mut self) -> ImageSurface {
        self.cache_content.clone()
    }
}
impl Drop for RingCtx {
    fn drop(&mut self) {
        if let Some(r) = self.runner.take() {
            r.close();
        }
    }
}

pub fn init_ring(
    expose: &BoxExposeRc,
    ring_width: f64,
    radius: f64,
    bg_color: RGBA,
    fg_color: RGBA,
) -> RingCtx {
    let ring = Rc::new(RefCell::new(Ring::new(
        ring_width, radius, bg_color, fg_color,
    )));
    let re = {
        let expose = expose.borrow_mut();
        let s = expose.update_signal();
        RingEvents {
            queue_draw: Box::new(move || {
                s.force_send(());
            }),
        }
    };
    RingCtx::new(re, ring, Duration::from_millis(1000), Box::new(|| 1.))
}
