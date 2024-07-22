use std::{cell::RefCell, rc::Rc, time::Duration};

use cairo::{Format, ImageSurface};
use gtk::{
    gdk::RGBA,
    prelude::{GdkCairoContextExt, WidgetExt},
    DrawingArea,
};
use interval_task::runner::{ExternalRunnerExt, Runner, Task};

use crate::ui::draws::{shape::draw_fan, util::Z};

use super::wrapbox::display::grid::DisplayWidget;

#[derive(Debug)]
pub struct Ring {
    pub progress: f64,

    pub radius: f64,
    pub fg_color: RGBA,

    pub bg_arc: ImageSurface,
    pub inner_radius: f64,

    pub cache_content: ImageSurface,
}
impl Ring {
    pub fn new(ring_width: f64, radius: f64, bg_color: RGBA, fg_color: RGBA) -> Self {
        let progress = 0.5;
        let (bg_arc, inner_radius) = Self::draw_base(radius, ring_width, &bg_color);
        let cache_content = Self::draw_progress(&bg_arc, inner_radius, &fg_color, progress, radius);
        Self {
            progress,
            radius,
            fg_color,
            cache_content,
            bg_arc,
            inner_radius,
        }
    }
    pub fn update_progress(&mut self, p: f64) {
        if p != self.progress {
            self.progress = p;
            self.redraw()
        }
    }
    pub fn redraw(&mut self) {
        self.cache_content = Self::draw_progress(
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
    ) -> ImageSurface {
        let size = (bg_arc.width(), bg_arc.height());
        let surf = ImageSurface::create(Format::ARgb32, size.0, size.1).unwrap();
        let ctx = cairo::Context::new(&surf).unwrap();

        ctx.set_source_surface(bg_arc, Z, Z).unwrap();
        ctx.rectangle(Z, Z, size.0 as f64, size.1 as f64);
        ctx.fill().unwrap();

        ctx.set_source_color(fg_color);
        draw_fan(&ctx, (radius, radius), radius, 0., progress * 2.);
        ctx.fill().unwrap();

        ctx.set_operator(cairo::Operator::Clear);
        draw_fan(&ctx, (radius, radius), inner_radius, 0., 2.);
        ctx.fill().unwrap();
        surf
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

pub struct RingCtx {
    inner: Rc<RefCell<Ring>>,
    runner: Option<Runner<Task>>,
}
impl RingCtx {
    pub fn new(
        darea: &DrawingArea,
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
            use gtk::glib;
            glib::spawn_future_local(glib::clone!(
                #[weak]
                darea,
                #[weak]
                inner,
                async move {
                    while let Ok(res) = r.recv().await {
                        inner.borrow_mut().update_progress(res);
                        darea.queue_draw();
                    }
                    log::warn!("ring update runner closed");
                }
            ));
            Some(runner)
        };
        Self { inner, runner }
    }
}

impl DisplayWidget for RingCtx {
    fn get_size(&mut self) -> (f64, f64) {
        let c = &self.inner.borrow_mut().cache_content;
        (c.width() as f64, c.height() as f64)
    }

    fn content(&mut self) -> ImageSurface {
        self.inner.borrow_mut().cache_content.clone()
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
    darea: &DrawingArea,
    ring_width: f64,
    radius: f64,
    bg_color: RGBA,
    fg_color: RGBA,
) -> RingCtx {
    let ring = Rc::new(RefCell::new(Ring::new(
        ring_width, radius, bg_color, fg_color,
    )));
    RingCtx::new(darea, ring, Duration::from_millis(1000), Box::new(|| 1.))
}
