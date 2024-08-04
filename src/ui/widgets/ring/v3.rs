/// NOTE: This widget can not be used directly
use std::cell::Cell;
use std::{cell::RefCell, rc::Rc, time::Duration};

use cairo::{Format, ImageSurface, ImageSurfaceDataOwned};
use educe::Educe;
use gtk::glib;
use gtk::pango::Layout;
use gtk::{gdk::RGBA, prelude::GdkCairoContextExt};
use interval_task::runner::{ExternalRunnerExt, Runner, Task};

use crate::config::widgets::ring::{RingConfig, RingPreset};
use crate::plug::system::{
    get_battery_info, get_cpu_info, get_disk_info, get_ram_info, get_swap_info,
};
use crate::ui::draws::frame_manager::FrameManager;
use crate::ui::draws::mouse_state::MouseEvent;
use crate::ui::draws::transition_state::{self, TransitionState, TransitionStateRc};
use crate::ui::draws::util::new_surface;
use crate::ui::draws::{shape::draw_fan, util::Z};

use super::super::wrapbox::display::grid::DisplayWidget;
use super::super::wrapbox::expose::BoxExposeRc;

fn draw_text(pl: &Layout, color: &RGBA, text: &str) -> ImageSurface {
    pl.set_text(text);
    let size = pl.pixel_size();
    let surf = new_surface(size);
    let ctx = cairo::Context::new(&surf).unwrap();
    ctx.set_antialias(cairo::Antialias::None);
    ctx.set_source_color(color);
    pangocairo::functions::show_layout(&ctx, pl);
    surf
}

fn from_kb(total: u64, avaibale: u64) -> (f64, f64, &'static str) {
    let mut c = 0;
    let mut total = total as f64;
    let mut avaibale = avaibale as f64;
    while total > 1000. && c < 3 {
        total /= 1000.;
        avaibale /= 1000.;
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
fn from_kib(total: u64, avaibale: u64) -> (f64, f64, &'static str) {
    let mut c = 0;
    let mut total = total as f64;
    let mut avaibale = avaibale as f64;
    while total > 1024. && c < 3 {
        total /= 1024.;
        avaibale /= 1024.;
        c += 1;
    }
    let surfix = match c {
        0 => "KiB",
        1 => "MiB",
        2 => "GiB",
        3 => "TiB",
        _ => unreachable!(),
    };
    (total, avaibale, surfix)
}

#[derive(Educe)]
#[educe(Debug)]
pub struct Ring {
    // config
    pub radius: f64,
    pub fg_color: RGBA,

    // from base
    pub bg_arc: ImageData,
    pub inner_radius: f64,
    // pub layout: Layout,
    pub prefix_text: Option<ImageData>,
}
impl Ring {
    pub fn new(config: &RingConfig) -> Self {
        let radius = config.common.radius;
        let ring_width = config.common.ring_width;
        let bg_color = config.common.bg_color;
        let fg_color = config.common.fg_color;
        let prefix = config.common.prefix.clone();
        let font_family = config.common.font_family.clone();
        let (layout, prefix_text, bg_arc, inner_radius) = Self::draw_base(
            radius,
            ring_width,
            &bg_color,
            &fg_color,
            prefix,
            font_family,
        );

        Self {
            radius,
            fg_color,
            bg_arc: bg_arc.into(),
            inner_radius,
            layout,
            prefix_text: prefix_text.map(|p| p.into()),
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn draw_progress(&self, progress: f64, text: Option<String>) -> ProgressCache {
        let ring_surf = {
            let radius = self.radius;
            let mut size = (self.bg_arc.width, self.bg_arc.height);
            if let Some(img) = &self.prefix_text {
                size.0 += img.width
            }
            let surf = ImageSurface::create(Format::ARgb32, size.0, size.1).unwrap();
            let ctx = cairo::Context::new(&surf).unwrap();

            if let Some(img) = &self.prefix_text {
                let tranaslate_x = img.width;
                ctx.set_source_surface(img, Z, Z).unwrap();
                ctx.paint().unwrap();
                ctx.translate(tranaslate_x as f64, Z);
            }

            ctx.set_source_surface(&self.bg_arc, Z, Z).unwrap();
            ctx.paint().unwrap();

            ctx.set_source_color(&self.fg_color);
            draw_fan(&ctx, (radius, radius), radius, -0.5, progress * 2. - 0.5);
            ctx.fill().unwrap();

            ctx.set_operator(cairo::Operator::Clear);
            draw_fan(&ctx, (radius, radius), self.inner_radius, 0., 2.);
            ctx.fill().unwrap();
            surf
        };

        let text_surf = {
            if let Some(text) = text {
                Some(draw_text(&self.layout, &self.fg_color, text.as_str()).into())
            } else {
                None
            }
        };

        ProgressCache {
            prefix_ring: ring_surf.into(),
            text: text_surf,
        }
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

#[derive(Educe)]
#[educe(Debug)]
struct ImageData {
    width: i32,
    height: i32,
    stride: i32,
    format: Format,
    #[educe(Debug(ignore))]
    data: Vec<u8>,
}
unsafe impl Send for ImageData {}
impl From<ImageSurface> for ImageData {
    fn from(value: ImageSurface) -> Self {
        Self {
            width: value.width(),
            height: value.height(),
            stride: value.stride(),
            format: value.format(),
            data: value.take_data().unwrap().to_vec(),
        }
    }
}
impl Into<ImageSurface> for ImageData {
    fn into(self) -> ImageSurface {
        ImageSurface::create_for_data(self.data, self.format, self.width, self.height, self.stride)
            .unwrap()
    }
}

struct ProgressCache {
    prefix_ring: ImageData,
    text: Option<ImageData>,
}
unsafe impl Send for ProgressCache {}

fn parse_preset(
    preset: RingPreset,
    inner: Ring,
) -> (
    u64,
    Box<dyn Send + FnMut() -> Result<ProgressCache, String>>,
) {
    match preset {
        crate::config::widgets::ring::RingPreset::Ram => (
            1000,
            Box::new(|| {
                let (progress, text) = if let Some((ava, total)) = get_ram_info() {
                    let (total, avaibale, surfix) = from_kib(total, ava);
                    let progress = avaibale / total;
                    let text = Some(format!(
                        " {:.2}{surfix} / {:.2}{surfix} [{:.2}%]",
                        avaibale,
                        total,
                        progress * 100.
                    ));
                    (progress, text)
                } else {
                    (0., None)
                };

                Ok(inner.draw_progress(progress, text))
            }),
        ),
        crate::config::widgets::ring::RingPreset::Swap => (
            1000,
            Box::new(|| {
                let (progress, text) = if let Some((ava, total)) = get_swap_info() {
                    let (total, avaibale, surfix) = from_kib(total, ava);
                    let progress = avaibale / total;
                    let text = Some(format!(
                        " {:.2}{surfix} / {:.2}{surfix} [{:.2}%]",
                        avaibale,
                        total,
                        progress * 100.
                    ));
                    (progress, text)
                } else {
                    (0., None)
                };

                Ok(inner.draw_progress(progress, text))
            }),
        ),
        crate::config::widgets::ring::RingPreset::Cpu => (
            1000,
            Box::new(|| {
                let (progress, text) = if let Some((progress, temp)) = get_cpu_info() {
                    let text = Some(format!(" {:.2}% {temp:.2}°C", progress * 100.));
                    (progress, text)
                } else {
                    (0., None)
                };

                Ok(inner.draw_progress(progress, text))
            }),
        ),
        crate::config::widgets::ring::RingPreset::Battery => (
            1000,
            Box::new(|| {
                let (progress, text) = if let Some(progress) = get_battery_info() {
                    let text = Some(format!(" {:.2}%", progress * 100.));
                    (progress, text)
                } else {
                    (0., None)
                };

                Ok(inner.draw_progress(progress, text))
            }),
        ),
        crate::config::widgets::ring::RingPreset::Disk(s) => {
            // let s = s.clone();
            (
                1000,
                Box::new(move || {
                    let (progress, text) = if let Some((ava, total)) = get_disk_info(s.as_str()) {
                        let (total, avaibale, surfix) = from_kib(total, ava);
                        let progress = avaibale / total;
                        let text = Some(format!(
                            " [Partition: {}] {:.2}{surfix} / {:.2}{surfix} [{:.2}%]",
                            s,
                            avaibale,
                            total,
                            progress * 100.
                        ));
                        (progress, text)
                    } else {
                        (0., None)
                    };

                    Ok(inner.draw_progress(progress, text))
                }),
            )
        }
        crate::config::widgets::ring::RingPreset::Custom(mut f) => {
            if let Some((ms, f)) = f.update_with_interval_ms.take() {
                (
                    ms,
                    Box::new(move || {
                        let (progress, text) = f()?;
                        Ok(inner.draw_progress(progress, text))
                    }),
                )
            } else {
                (999999, Box::new(|| Ok(inner.draw_progress(0., None))))
            }
        }
    }
}

pub struct RingCtx {
    cache_content: Rc<Cell<ImageSurface>>,
    #[allow(dead_code)]
    runner: Option<Runner<Task>>,
    text_ts: TransitionStateRc,
    events: RingEvents,
}
impl RingCtx {
    fn new(mut events: RingEvents, config: RingConfig) -> Result<Self, String> {
        let mut update_ctx = {
            let inner = Ring::new(&config);
            parse_preset(config.preset, inner)
        };

        let text_ts = Rc::new(RefCell::new(TransitionState::new(Duration::from_millis(
            config.common.text_transition_ms,
        ))));

        let (cache_content, progress_cache) = {
            let cache = update_ctx.1()?;
            (
                Rc::new(Cell::new(Self::_combine(
                    &cache.prefix_ring,
                    &cache.text,
                    text_ts.borrow().get_y(),
                ))),
                Rc::new(Cell::new(cache)),
            )
        };

        let (ring_update_signal_sender, ring_update_signal_receiver) = async_channel::bounded(1);
        let send_ring_redraw_signal_weak = {
            let w = ring_update_signal_sender.downgrade();
            Box::new(move || {
                if let Some(s) = w.upgrade() {
                    s.force_send(()).ok();
                }
            })
        };

        // ring cache redraw signal
        let send_ring_redraw_signal = Box::new(move || {
            // ignored result
            let _ = ring_update_signal_sender.force_send(());
        });

        // just use separate threads to run rather than one async thread.
        // incase some task takes too many time.
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
                #[strong]
                progress_cache,
                async move {
                    while let Ok(res) = r.recv().await {
                        progress_cache.set(res.unwrap());
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

        // it's a while loop inside async block, so no matter weak or strong
        glib::spawn_future_local(glib::clone!(
            #[strong]
            text_ts,
            #[strong]
            cache_content,
            #[strong]
            progress_cache,
            async move {
                while ring_update_signal_receiver.recv().await.is_ok() {
                    let y = text_ts.borrow().get_y();
                    let progress_cache = unsafe { progress_cache.as_ptr().as_ref().unwrap() };
                    cache_content.set(Self::_combine(
                        &progress_cache.prefix_ring,
                        &progress_cache.text,
                        y,
                    ));
                    ensure_fm(y);
                    queue_draw()
                }
                log::warn!("ring update runner closed");
            }
        ));

        Ok(Self {
            runner,
            cache_content,
            text_ts,
            events,
        })
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
                self.text_ts
                    .borrow_mut()
                    .set_direction_self(transition_state::TransitionDirection::Forward);
                (self.events.queue_draw)()
            }
            MouseEvent::Leave => {
                self.text_ts
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

pub fn init_ring(expose: &BoxExposeRc, config: RingConfig) -> Result<RingCtx, String> {
    let re = {
        let expose = expose.borrow_mut();
        let s = expose.update_func();
        RingEvents {
            queue_draw: Box::new(s),
        }
    };
    RingCtx::new(re, config)
}
