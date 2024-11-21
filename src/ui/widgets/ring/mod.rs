mod draw;

use std::cell::Cell;
use std::{cell::RefCell, rc::Rc, time::Duration};

use async_channel::Sender;
use cairo::ImageSurface;
use draw::{Ring, RingCache};
use gtk::glib;
use interval_task::runner::Runner;

use crate::config::widgets::wrapbox::ring::{RingConfig, RingPreset};
use crate::plug::system::{
    get_battery_info, get_cpu_info, get_disk_info, get_ram_info, get_swap_info,
};
use crate::ui::draws::frame_manager::FrameManager;
use crate::ui::draws::mouse_state::MouseEvent;
use crate::ui::draws::transition_state::{self, TransitionState, TransitionStateRc};
use crate::ui::draws::util::{horizon_center_combine, new_surface, Z};

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

type RunnerTask = Box<dyn Send + FnMut(&mut Ring) -> Result<RingCache, String>>;
fn parse_preset(preset: &mut RingPreset) -> (u64, RunnerTask) {
    match preset {
        crate::config::widgets::wrapbox::ring::RingPreset::Ram => (
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

                Ok(inner.draw_ring(progress, text))
            }),
        ),
        crate::config::widgets::wrapbox::ring::RingPreset::Swap => (
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

                Ok(inner.draw_ring(progress, text))
            }),
        ),
        crate::config::widgets::wrapbox::ring::RingPreset::Cpu => (
            1000,
            Box::new(|inner| {
                let (progress, text) = if let Some((progress, temp)) = get_cpu_info() {
                    let text = Some(format!(" {:.2}% {temp:.2}°C", progress * 100.));
                    (progress, text)
                } else {
                    (0., None)
                };

                Ok(inner.draw_ring(progress, text))
            }),
        ),
        crate::config::widgets::wrapbox::ring::RingPreset::Battery => (
            1000,
            Box::new(|inner| {
                let (progress, text) = if let Some(progress) = get_battery_info() {
                    let text = Some(format!(" {:.2}%", progress * 100.));
                    (progress, text)
                } else {
                    (0., None)
                };

                Ok(inner.draw_ring(progress, text))
            }),
        ),
        crate::config::widgets::wrapbox::ring::RingPreset::Disk(s) => {
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

                    Ok(inner.draw_ring(progress, text))
                }),
            )
        }
        crate::config::widgets::wrapbox::ring::RingPreset::Custom(f) => {
            if let Some((ms, mut f)) = f.update_with_interval_ms.take() {
                (
                    ms,
                    Box::new(move |inner| {
                        let progress = f()?;
                        Ok(inner.draw_ring(progress, None))
                    }),
                )
            } else {
                (999999, Box::new(|inner| Ok(inner.draw_ring(0., None))))
            }
        }
    }
}

type RingUpdateSignal = Option<RingCache>;

pub struct RingCtx {
    cache_content: Rc<Cell<ImageSurface>>,
    runner: Runner<Ring>,

    pop_ts: TransitionStateRc,

    ring_update_signal_sender: Sender<RingUpdateSignal>,
}
impl RingCtx {
    fn new(events: RingEvents, mut config: RingConfig) -> Result<Self, String> {
        // preset: `update interval duration` & `update function`
        let update_ctx = parse_preset(&mut config.preset);

        // for text pop transition
        let pop_ts = Rc::new(RefCell::new(TransitionState::new(Duration::from_millis(
            config.common.text_transition_ms,
        ))));

        // ring content cache redraw signal
        let (ring_update_signal_sender, ring_update_signal_receiver) =
            async_channel::bounded::<RingUpdateSignal>(1);

        // frame manager
        let mut ensure_fm = {
            let update_signal = ring_update_signal_sender.clone();
            let mut fm = FrameManager::new(config.common.frame_rate, move || {
                // ignore result
                let _ = update_signal.try_send(None);
            });
            move |y| {
                if transition_state::is_in_transition(y) {
                    fm.start().unwrap();
                } else {
                    fm.stop().unwrap();
                }
            }
        };

        let prefix_hide = config.common.prefix_hide;
        let suffix_hide = config.common.suffix_hide;

        // just use separate threads to run rather than one async thread.
        // incase some task takes too much of cpu time.
        // let (runner, cache_content, mut ring_cache) = {
        let runner = {
            let update_signal = ring_update_signal_sender.clone();
            let mut uf = update_ctx.1;
            // NOTE: one thread each ring widget
            let mut runner = interval_task::runner::new_runner(
                Duration::from_millis(update_ctx.0),
                move || Ring::new(&mut config),
                move |ring| {
                    let res = uf(ring);
                    if let Ok(res) = res {
                        let _ = update_signal.force_send(Some(res));
                    }
                    false
                },
            );
            // start progress update interval thread
            runner.start().unwrap();

            runner
        };

        // wait for first progress
        let (cache_content, mut ring_cache) =
            if let Ok(Some(cache)) = ring_update_signal_receiver.recv_blocking() {
                let content = cache.merge(prefix_hide, suffix_hide, 0.);

                (Rc::new(Cell::new(content)), cache)
            } else {
                return Err("first frame fail to create".to_string());
            };

        let mut queue_draw = events.queue_draw;
        // it's a while loop inside async block, so no matter weak or strong
        glib::spawn_future_local(glib::clone!(
            #[strong]
            pop_ts,
            #[strong]
            cache_content,
            async move {
                while let Ok(res) = ring_update_signal_receiver.recv().await {
                    // refresh transition
                    pop_ts.borrow_mut().refresh();

                    // if new progress cache drawed, replace
                    if let Some(new_cache) = res {
                        ring_cache = new_cache;
                    }
                    let y = pop_ts.borrow().get_y();
                    cache_content.set(ring_cache.merge(prefix_hide, suffix_hide, y));
                    ensure_fm(y);
                    queue_draw()
                }
                log::debug!("ring update runner closed");
            }
        ));

        Ok(Self {
            runner,
            cache_content,

            pop_ts,

            ring_update_signal_sender,
        })
    }

    // fn _combine(r: &ImageSurface, t: Option<&ImageSurface>, y: f64) -> ImageSurface {
    //     if let Some(text) = t {
    //         let visible_text_width =
    //             transition_state::calculate_transition(y, (0., text.width() as f64));
    //         let text_visible_surf = new_surface((visible_text_width.ceil() as i32, text.height()));
    //         let ctx = cairo::Context::new(&text_visible_surf).unwrap();
    //         ctx.translate(-(text.width() - visible_text_width.ceil() as i32) as f64, Z);
    //         ctx.set_source_surface(text, Z, Z).unwrap();
    //         ctx.paint().unwrap();
    //         horizon_center_combine(r, &text_visible_surf)
    //     } else {
    //         r.clone()
    //     }
    // }
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
                self.pop_ts
                    .borrow_mut()
                    .set_direction_self(transition_state::TransitionDirection::Forward);
                // ignore
                let _ = self.ring_update_signal_sender.try_send(None);
                // let _ = self.ring_update_signal_sender.force_send(());
            }
            MouseEvent::Leave => {
                self.pop_ts
                    .borrow_mut()
                    .set_direction_self(transition_state::TransitionDirection::Backward);
                // ignore
                let _ = self.ring_update_signal_sender.try_send(None);
                // let _ = self.ring_update_signal_sender.force_send(());
            }
            _ => {}
        }
    }
}
impl Drop for RingCtx {
    fn drop(&mut self) {
        log::info!("drop ring ctx");
        self.ring_update_signal_sender.close();
        std::mem::take(&mut self.runner).close().unwrap();
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
