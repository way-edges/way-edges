use std::cell::Cell;
/// NOTE: This widget can not be used directly
use std::{cell::RefCell, rc::Rc, time::Duration};

use cairo::{Format, ImageSurface};
use educe::Educe;
use gtk::glib;
use gtk::pango::Layout;
use gtk::{gdk::RGBA, prelude::GdkCairoContextExt};
use interval_task::runner::{ExternalRunnerExt, Runner, Task};

use crate::config::widgets::ring::RingConfig;
use crate::config::widgets::slide::UpdateTask;
use crate::plug::system::{get_ram_info, get_swap_info};
use crate::ui::draws::frame_manager::FrameManager;
use crate::ui::draws::mouse_state::MouseEvent;
use crate::ui::draws::transition_state::{self, TransitionState, TransitionStateRc};
use crate::ui::draws::util::new_surface;
use crate::ui::draws::{shape::draw_fan, util::Z};

use super::wrapbox::display::grid::DisplayWidget;
use super::wrapbox::BoxExposeRc;

fn draw_text(size: (i32, i32), pl: &Layout, color: &RGBA) -> ImageSurface {
    let surf = new_surface(size);
    let ctx = cairo::Context::new(&surf).unwrap();
    ctx.set_antialias(cairo::Antialias::None);
    ctx.set_source_color(color);
    pangocairo::functions::show_layout(&ctx, &pl);
    surf
}

fn from_kb(total: u64, avaibale: u64) -> (f64, f64, &'static str) {
    let mut c = 0;
    let mut total = total as f64;
    let mut avaibale = avaibale as f64;
    while total > 1024. && c < 3 {
        total /= 1024.;
        avaibale /= 1024.;
        c += 1;
    }
    let surfix = match c {
        0 => "KB",
        1 => "MB",
        2 => "GB",
        3 => "TB",
        _ => unreachable!(),
    };
    (total, avaibale, surfix)
}

struct RamTextRender;
impl RamTextRender {
    fn core(&self, p: (u64, u64), pl: &Layout, color: &RGBA) -> ImageSurface {
        let (total, avaibale, surfix) = from_kb(p.1, p.0);
        pl.set_text(
            format!(
                " {:.2}{surfix} / {:.2}{surfix} [{:.2}%]",
                avaibale,
                total,
                avaibale / total * 100.
            )
            .as_str(),
        );
        let size = pl.pixel_size();
        draw_text(size, pl, color)
    }
}
impl TextRender for RamTextRender {
    fn draw_text(&self, _: f64, pl: &Layout, color: &RGBA) -> Option<ImageSurface> {
        get_ram_info().map(|p| self.core(p, pl, color))
    }
}

struct SwapTextRender;
impl TextRender for SwapTextRender {
    fn draw_text(&self, _: f64, pl: &Layout, color: &RGBA) -> Option<ImageSurface> {
        let a = get_swap_info().map(|p| RamTextRender.core(p, pl, color));
        a
    }
}

struct CustomTextRender {
    template: String,
}
impl TextRender for CustomTextRender {
    fn draw_text(&self, progress: f64, pl: &Layout, color: &RGBA) -> Option<ImageSurface> {
        const RING_TEMPLATE_PROGRESS_PLACEHOLDER: &str = "{progress}";
        let t = self.template.replace(
            RING_TEMPLATE_PROGRESS_PLACEHOLDER,
            format!("{:.0}%", progress * 100.).as_str(),
        );
        pl.set_text(t.as_str());
        let size = pl.pixel_size();
        Some(draw_text(size, pl, color))
    }
}

trait TextRender {
    fn draw_text(&self, progress: f64, pl: &Layout, color: &RGBA) -> Option<ImageSurface>;
}

#[derive(Educe)]
#[educe(Debug)]
pub struct Ring {
    pub progress: f64,

    // config
    pub radius: f64,
    pub fg_color: RGBA,
    #[educe(Debug(ignore))]
    text_render: Option<Box<dyn TextRender>>,

    // from base
    pub bg_arc: ImageSurface,
    pub inner_radius: f64,
    pub layout: Layout,
    pub prefix_text: Option<ImageSurface>,

    // progress redraw
    pub ring_surf: ImageSurface,
    pub text_surf: Option<ImageSurface>,
}
impl Ring {
    pub fn new(config: &RingConfig) -> Self {
        let radius = config.common.radius;
        let ring_width = config.common.ring_width;
        let bg_color = config.common.bg_color;
        let fg_color = config.common.fg_color;
        let prefix = config.common.prefix.clone();
        let text_render: Option<Box<dyn TextRender>> = match &config.preset {
            crate::config::widgets::ring::RingPreset::Ram => Some(Box::new(RamTextRender)),
            crate::config::widgets::ring::RingPreset::Swap => Some(Box::new(SwapTextRender)),
            crate::config::widgets::ring::RingPreset::Custom(c) => {
                if let Some(t) = &c.template {
                    Some(Box::new(CustomTextRender {
                        template: t.clone(),
                    }))
                } else {
                    None
                }
            }
            crate::config::widgets::ring::RingPreset::Cpu => todo!(),
            crate::config::widgets::ring::RingPreset::Battery => todo!(),
            crate::config::widgets::ring::RingPreset::Disk => todo!(),
        };
        let font_family = config.common.font_family.clone();
        let progress = 0.;
        let (layout, prefix_text, bg_arc, inner_radius) = Self::draw_base(
            radius,
            ring_width,
            &bg_color,
            &fg_color,
            prefix,
            font_family,
        );
        let (ring_surf, text_surf) = Self::draw_progress(
            &bg_arc,
            inner_radius,
            &fg_color,
            progress,
            radius,
            &prefix_text,
            &layout,
            &text_render,
        );

        Self {
            progress,
            radius,
            fg_color,
            bg_arc,
            inner_radius,
            ring_surf,
            text_surf,
            layout,
            prefix_text,
            text_render,
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
            &self.prefix_text,
            &self.layout,
            &self.text_render,
        );
    }
    #[allow(clippy::too_many_arguments)]
    pub fn draw_progress(
        bg_arc: &ImageSurface,
        inner_radius: f64,
        fg_color: &RGBA,
        progress: f64,
        radius: f64,
        prefix_text: &Option<ImageSurface>,
        layout: &Layout,
        text_render: &Option<Box<dyn TextRender>>,
    ) -> (ImageSurface, Option<ImageSurface>) {
        let ring_surf = {
            let mut size = (bg_arc.width(), bg_arc.height());
            if let Some(img) = &prefix_text {
                size.0 += img.width()
            }
            let surf = ImageSurface::create(Format::ARgb32, size.0, size.1).unwrap();
            let ctx = cairo::Context::new(&surf).unwrap();

            if let Some(img) = prefix_text {
                let tranaslate_x = img.width();
                ctx.set_source_surface(img, Z, Z).unwrap();
                ctx.paint().unwrap();
                ctx.translate(tranaslate_x as f64, Z);
            }

            ctx.set_source_surface(bg_arc, Z, Z).unwrap();
            ctx.paint().unwrap();

            ctx.set_source_color(fg_color);
            draw_fan(&ctx, (radius, radius), radius, -0.5, progress * 2. - 0.5);
            ctx.fill().unwrap();

            ctx.set_operator(cairo::Operator::Clear);
            draw_fan(&ctx, (radius, radius), inner_radius, 0., 2.);
            ctx.fill().unwrap();
            surf
        };

        let text_surf = {
            if let Some(t) = text_render {
                t.draw_text(progress, layout, fg_color)
            } else {
                None
            }
        };

        (ring_surf, text_surf)
    }
    pub fn draw_base(
        radius: f64,
        ring_width: f64,
        bg_color: &RGBA,
        fg_color: &RGBA,
        prefix: Option<String>,
        font_family: Option<String>,
    ) -> (Layout, Option<ImageSurface>, ImageSurface, f64) {
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

        let (ly, prefix_img) = {
            let pc = pangocairo::pango::Context::new();
            let fm = pangocairo::FontMap::default();
            pc.set_font_map(Some(&fm));
            let mut desc = pc.font_description().unwrap();
            desc.set_absolute_size(radius * 1.5 * 1024.);
            if let Some(font_family) = font_family {
                desc.set_family(font_family.as_str());
                pc.set_font_description(Some(&desc));
            }
            // desc.set_family("JetBrainsMono Nerd Font Mono");
            let pl = pangocairo::pango::Layout::new(&pc);

            if let Some(prefix) = prefix {
                pl.set_text(prefix.as_str());
                let size = pl.pixel_size();

                let surf = ImageSurface::create(Format::ARgb32, size.0, size.1).unwrap();
                let ctx = cairo::Context::new(&surf).unwrap();
                ctx.set_antialias(cairo::Antialias::None);

                ctx.set_source_color(fg_color);
                pangocairo::functions::show_layout(&ctx, &pl);
                (pl, Some(surf))
            } else {
                (pl, None)
            }
        };

        (ly, prefix_img, bg_surf, small_radius)
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
        let update_ctx: (u64, Box<dyn Send + Sync + FnMut() -> Result<f64, String>>) =
            match &mut config.preset {
                crate::config::widgets::ring::RingPreset::Ram => (
                    1000,
                    Box::new(|| {
                        if let Some((ava, total)) = get_ram_info() {
                            Ok(ava as f64 / total as f64)
                        } else {
                            Ok(0.)
                        }
                    }),
                ),
                crate::config::widgets::ring::RingPreset::Swap => (
                    1000,
                    Box::new(|| {
                        if let Some((ava, total)) = get_swap_info() {
                            Ok(ava as f64 / total as f64)
                        } else {
                            Ok(0.)
                        }
                    }),
                ),
                crate::config::widgets::ring::RingPreset::Custom(f) => {
                    if let Some((ms, f)) = f.update_with_interval_ms.take() {
                        (ms, f)
                    } else {
                        (999999, Box::new(|| Ok(0.)))
                    }
                }
                crate::config::widgets::ring::RingPreset::Cpu => todo!(),
                crate::config::widgets::ring::RingPreset::Battery => todo!(),
                crate::config::widgets::ring::RingPreset::Disk => todo!(),
            };
        let inner = Rc::new(RefCell::new(Ring::new(&config)));
        let text_ts = TransitionState::new(Duration::from_millis(config.common.text_transition_ms));
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

        let mut ensure_fm = {
            let update_func = send_ring_redraw_signal_weak;
            let mut fm = FrameManager::new(config.common.frame_rate, move || {
                update_func();
            });
            move |y| {
                if transition_state::is_in_transition(y) {
                    fm.start().unwrap();
                } else {
                    fm.stop().unwrap();
                }
            }
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
                    ensure_fm(y);
                    queue_draw()
                }
                log::warn!("ring update runner closed");
            }
        ));

        Self {
            inner,
            runner,
            cache_content,
            ts,
            events,
        }
    }

    fn _combine(r: &ImageSurface, t: &Option<ImageSurface>, y: f64) -> ImageSurface {
        let ring_size = (r.width(), r.height());
        let (text_size, visible_text_width, size) = {
            if let Some(t) = t {
                let text_size = (t.width(), t.height());
                let visible_text_width =
                    transition_state::calculate_transition(y, (0., text_size.0 as f64));
                let size = (
                    ring_size.0 + visible_text_width.ceil() as i32,
                    ring_size.1.max(text_size.1),
                );
                (text_size, visible_text_width, size)
            } else {
                let size = (ring_size.0, ring_size.1);
                ((0, 0), Z, size)
            }
        };
        // let text_size = (t.width(), t.height());
        // let visible_text_width =
        //     transition_state::calculate_transition(y, (0., text_size.0 as f64));
        // let size = (
        //     ring_size.0 + visible_text_width.ceil() as i32,
        //     ring_size.1.max(text_size.1),
        // );

        let surf = ImageSurface::create(Format::ARgb32, size.0, size.1).unwrap();
        let ctx = cairo::Context::new(&surf).unwrap();
        ctx.set_antialias(cairo::Antialias::None);
        ctx.set_source_surface(r, Z, Z).unwrap();
        ctx.paint().unwrap();

        if let Some(t) = t {
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
        }

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
