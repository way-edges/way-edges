use std::cell::Cell;
/// NOTE: This widget can not be used directly
use std::{cell::RefCell, rc::Rc, time::Duration};

use cairo::{Format, ImageSurface};
use gtk::glib;
use gtk::{gdk::RGBA, prelude::GdkCairoContextExt};
use interval_task::runner::{ExternalRunnerExt, Runner, Task};

use crate::config::widgets::ring::RingConfig;
use crate::ui::draws::frame_manager::FrameManager;
use crate::ui::draws::mouse_state::MouseEvent;
use crate::ui::draws::transition_state::{self, TransitionState, TransitionStateRc};
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
    pub fn new(config: &RingConfig) -> Self {
        let radius = config.radius;
        let ring_width = config.ring_width;
        let bg_color = config.bg_color;
        let fg_color = config.fg_color;
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
            let size = pl.pixel_size();

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
impl Drop for Ring {
    fn drop(&mut self) {
        log::debug!("drop ring");
    }
}

struct RingEvents {
    queue_draw: Box<dyn FnMut() + 'static>,
}

pub struct RingCtx {
    cache_content: Rc<Cell<ImageSurface>>,
    inner: Rc<RefCell<Ring>>,
    runner: Option<Runner<Task>>,
    ts: TransitionStateRc,
    events: RingEvents,
}
impl RingCtx {
    fn new(mut events: RingEvents, mut config: RingConfig) -> Self {
        let update_ctx = config
            .update_with_interval_ms
            .take()
            .unwrap_or((999999, Box::new(|| Ok(0.))));
        let inner = Rc::new(RefCell::new(Ring::new(&config)));
        let text_ts = TransitionState::new(Duration::from_millis(config.text_transition_ms));
        let cache_content = {
            let a = inner.borrow();
            Rc::new(Cell::new(Self::_combine(
                &a.ring_surf,
                &a.text_surf,
                text_ts.get_y(),
            )))
        };
        let ts = Rc::new(RefCell::new(text_ts));
        let (ring_update_signal_sender, ring_update_signal_receiver) = async_channel::bounded(1);
        let send_ring_redraw_signal_weak = {
            let w = ring_update_signal_sender.downgrade();
            Box::new(move || {
                if let Some(s) = w.upgrade() {
                    s.force_send(()).ok();
                }
            })
        };
        let send_ring_redraw_signal = Box::new(move || {
            // ignored result
            ring_update_signal_sender.force_send(()).ok();
        });
        let runner = {
            let mut runner = interval_task::runner::new_external_close_runner(
                Duration::from_millis(update_ctx.0),
            );
            let (s, r) = async_channel::bounded(1);
            let mut uf = update_ctx.1;
            runner.set_task(Box::new(move || {
                let res = uf();
                s.force_send(res).ok();
            }));
            let redraw = send_ring_redraw_signal.clone();
            glib::spawn_future_local(glib::clone!(
                #[weak]
                inner,
                async move {
                    while let Ok(res) = r.recv().await {
                        inner.borrow_mut().update_progress(res.unwrap());
                        redraw();
                    }
                    log::warn!("ring progress runner closed");
                }
            ));
            runner.start().unwrap();
            Some(runner)
        };
        {
            let mut fm = {
                let update_func = send_ring_redraw_signal_weak;
                FrameManager::new(config.frame_rate, move || {
                    update_func();
                })
            };
            let mut queue_draw = events.queue_draw;
            events.queue_draw = send_ring_redraw_signal;
            glib::spawn_future_local(glib::clone!(
                #[weak]
                inner,
                #[weak]
                ts,
                #[strong]
                cache_content,
                async move {
                    while ring_update_signal_receiver.recv().await.is_ok() {
                        let y = ts.borrow().get_y();
                        cache_content.set(Self::_combine(
                            &inner.borrow().ring_surf,
                            &inner.borrow().text_surf,
                            y,
                        ));
                        if transition_state::is_in_transition(y) {
                            fm.start().unwrap();
                        } else {
                            fm.stop().unwrap();
                        }
                        queue_draw()
                    }
                    log::warn!("ring update runner closed");
                }
            ));
        };

        Self {
            inner,
            runner,
            cache_content,
            ts,
            events,
        }
    }

    fn _combine(r: &ImageSurface, t: &ImageSurface, y: f64) -> ImageSurface {
        let ring_size = (r.width(), r.height());
        let text_size = (t.width(), t.height());
        let visible_text_width =
            transition_state::calculate_transition(y, (0., text_size.0 as f64));
        let size = (
            ring_size.0 + visible_text_width.ceil() as i32,
            ring_size.1.max(text_size.1),
        );

        let surf = ImageSurface::create(Format::ARgb32, size.0, size.1).unwrap();
        let ctx = cairo::Context::new(&surf).unwrap();
        ctx.set_antialias(cairo::Antialias::None);
        ctx.set_source_surface(r, Z, Z).unwrap();
        ctx.paint().unwrap();
        ctx.set_source_surface(
            t,
            -text_size.0 as f64 + ring_size.0 as f64 + visible_text_width,
            Z,
        )
        .unwrap();
        ctx.rectangle(
            ring_size.0 as f64,
            Z,
            text_size.0 as f64,
            text_size.1 as f64,
        );
        ctx.fill().unwrap();

        surf
    }
}

impl DisplayWidget for RingCtx {
    fn get_size(&mut self) -> (f64, f64) {
        let c = &unsafe { self.cache_content.as_ptr().as_ref().unwrap() };
        (c.width() as f64, c.height() as f64)
    }

    fn content(&mut self) -> ImageSurface {
        unsafe { self.cache_content.as_ptr().as_ref().unwrap().clone() }
    }
    fn on_mouse_event(&mut self, event: MouseEvent) {
        match event {
            MouseEvent::Enter(_) => {
                self.ts
                    .borrow_mut()
                    .set_direction_self(transition_state::TransitionDirection::Forward);
                (self.events.queue_draw)()
            }
            MouseEvent::Leave => {
                self.ts
                    .borrow_mut()
                    .set_direction_self(transition_state::TransitionDirection::Backward);
                (self.events.queue_draw)()
            }
            _ => {}
        }
    }
}
impl Drop for RingCtx {
    fn drop(&mut self) {
        log::debug!("drop ring ctx");
        if let Some(r) = self.runner.take() {
            gio::spawn_blocking(move || {
                r.close().unwrap();
            });
        }
    }
}

pub fn init_ring(expose: &BoxExposeRc, config: RingConfig) -> RingCtx {
    let re = {
        let expose = expose.borrow_mut();
        let s = expose.update_func();
        RingEvents {
            queue_draw: Box::new(s),
        }
    };
    RingCtx::new(re, config)
}
