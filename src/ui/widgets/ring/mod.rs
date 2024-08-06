/// NOTE: This widget can not be used directly
mod draw;

use std::cell::Cell;
use std::{cell::RefCell, rc::Rc, time::Duration};

use async_channel::Sender;
use cairo::{Format, ImageSurface};
use draw::{ProgressCache, Ring};
use gtk::glib;
use interval_task::runner::Runner;

use crate::config::widgets::ring::{RingConfig, RingPreset};
use crate::plug::system::{
    get_battery_info, get_cpu_info, get_disk_info, get_ram_info, get_swap_info,
};
use crate::ui::draws::frame_manager::FrameManager;
use crate::ui::draws::mouse_state::MouseEvent;
use crate::ui::draws::transition_state::{self, TransitionState, TransitionStateRc};
use crate::ui::draws::util::Z;

use super::wrapbox::display::grid::DisplayWidget;
use super::wrapbox::expose::BoxExposeRc;

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

struct RingEvents {
    queue_draw: Box<dyn FnMut() + 'static>,
}

type RunnerTask = Box<dyn Send + FnMut(&mut Ring) -> Result<ProgressCache, String>>;
fn parse_preset(preset: &mut RingPreset) -> (u64, RunnerTask) {
    match preset {
        crate::config::widgets::ring::RingPreset::Ram => (
            1000,
            Box::new(|inner| {
                let (progress, text) = if let Some([ava, total]) = get_ram_info() {
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
            Box::new(|inner| {
                let (progress, text) = if let Some([ava, total]) = get_swap_info() {
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
            Box::new(|inner| {
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
            Box::new(|inner| {
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
            let s = s.clone();
            (
                1000,
                Box::new(move |inner| {
                    let (progress, text) = if let Some((ava, total)) = get_disk_info(s.as_str()) {
                        let (total, avaibale, surfix) = from_kb(total, ava);
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
        crate::config::widgets::ring::RingPreset::Custom(f) => {
            if let Some((ms, mut f)) = f.update_with_interval_ms.take() {
                (
                    ms,
                    Box::new(move |inner| {
                        let (progress, text) = f()?;
                        Ok(inner.draw_progress(progress, text))
                    }),
                )
            } else {
                (999999, Box::new(|inner| Ok(inner.draw_progress(0., None))))
            }
        }
    }
}

pub struct RingCtx {
    cache_content: Rc<Cell<ImageSurface>>,
    runner: Option<Runner<Ring>>,
    text_ts: TransitionStateRc,
    ring_update_signal_sender: Sender<()>,
}
impl RingCtx {
    fn new(events: RingEvents, mut config: RingConfig) -> Result<Self, String> {
        let update_ctx = { parse_preset(&mut config.preset) };

        let text_ts = Rc::new(RefCell::new(TransitionState::new(Duration::from_millis(
            config.common.text_transition_ms,
        ))));

        let (ring_update_signal_sender, ring_update_signal_receiver) = async_channel::bounded(1);

        // ring cache redraw signal
        let make_redraw_send_func = || {
            let s = ring_update_signal_sender.clone();
            move || {
                // ignored result
                let _ = s.force_send(());
            }
        };

        let mut ensure_fm = {
            let update_func = make_redraw_send_func();
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

        // just use separate threads to run rather than one async thread.
        // incase some task takes too many time.
        let (runner, cache_content, progress_cache) = {
            let (s, r) = async_channel::bounded(1);
            let mut uf = update_ctx.1;
            // NOTE: one thread each ring widget
            let mut runner = interval_task::runner::new_runner(
                Duration::from_millis(update_ctx.0),
                move || Ring::new(&config),
                move |ring| {
                    let res = uf(ring);
                    s.force_send(res).ok();
                    false
                },
            );
            runner.start().unwrap();

            let (cache_content, progress_cache) = if let Ok(res) = r.recv_blocking() {
                let mut cache = res?;
                let prefix = unsafe { cache.prefix_ring.temp_surface() };
                let text = cache.text.as_mut().map(|d| unsafe { d.temp_surface() });
                (
                    Rc::new(Cell::new(Self::_combine(
                        &prefix,
                        text.as_ref(),
                        text_ts.borrow().get_y(),
                    ))),
                    Rc::new(Cell::new(cache)),
                )
            } else {
                return Err("first frame fail to create".to_string());
            };

            let redraw = make_redraw_send_func();
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
            (Some(runner), cache_content, progress_cache)
        };

        let mut queue_draw = events.queue_draw;
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
                    let (prefix, text) = {
                        let progress_cache = unsafe { progress_cache.as_ptr().as_mut().unwrap() };
                        let prefix = unsafe { progress_cache.prefix_ring.temp_surface() };
                        let text = progress_cache
                            .text
                            .as_mut()
                            .map(|d| unsafe { d.temp_surface() });
                        (prefix, text)
                    };
                    cache_content.set(Self::_combine(&prefix, text.as_ref(), y));
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
            ring_update_signal_sender,
        })
    }

    fn _combine(r: &ImageSurface, t: Option<&ImageSurface>, y: f64) -> ImageSurface {
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
                // ignore
                let _ = self.ring_update_signal_sender.force_send(());
            }
            MouseEvent::Leave => {
                self.text_ts
                    .borrow_mut()
                    .set_direction_self(transition_state::TransitionDirection::Backward);
                // ignore
                let _ = self.ring_update_signal_sender.force_send(());
            }
            _ => {}
        }
    }
}
impl Drop for RingCtx {
    fn drop(&mut self) {
        log::debug!("drop ring ctx");
        self.ring_update_signal_sender.close();
        if let Some(r) = self.runner.take() {
            r.close().unwrap();
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
