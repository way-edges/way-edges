// mod base;
mod draw;
mod event;
mod font;

use std::{
    cell::Cell,
    rc::{Rc, Weak},
    time::Duration,
};

use crate::window::WindowContext;
use cairo::ImageSurface;
use config::{
    widgets::slide::{base::SlideConfig, preset::CustomConfig},
    Config,
};
use gtk::{gdk::Monitor, prelude::MonitorExt};
use util::shell::shell_cmd;

pub fn init_widget(
    window: &mut WindowContext,
    monitor: &Monitor,
    config: Config,
    mut w_conf: SlideConfig,
) {
    let geom = monitor.geometry();
    let size = (geom.width(), geom.height());
    w_conf.size.calculate_relative(size, config.edge);

    use config::widgets::slide::preset::Preset;

    match std::mem::take(&mut w_conf.preset) {
        Preset::Speaker(pulse_audio_config) => todo!(),
        Preset::Microphone(pulse_audio_config) => todo!(),
        Preset::Backlight(backlight_config) => todo!(),
        Preset::Custom(custom_config) => custom(window, &config, w_conf, custom_config),
    }
}

fn custom(
    window: &mut WindowContext,
    config: &Config,
    mut w_conf: SlideConfig,
    preset_conf: CustomConfig,
) {
    // NOTE: THIS TYPE ANNOTATION IS WEIRD
    window.set_draw_func(None::<fn() -> Option<ImageSurface>>);

    let (_, draw_func) = draw::make_draw_func(&w_conf, config.edge);
    let draw_func = Rc::new(draw_func);
    let progress_cache = Rc::new(Cell::new(0.));
    if preset_conf.interval > 0 && !preset_conf.cmd.is_empty() {
        let (s, r) = async_channel::bounded(1);
        let cmd = preset_conf.cmd.clone();
        let runner = interval_task::runner::new_runner(
            Duration::from_millis(preset_conf.interval),
            || (),
            move |_| {
                match shell_cmd(&cmd).and_then(|res| {
                    use std::str::FromStr;
                    f64::from_str(res.trim()).map_err(|_| "Invalid number".to_string())
                }) {
                    Ok(progress) => {
                        s.force_send(progress).unwrap();
                    }
                    Err(err) => log::error!("slide custom updata error: {err}"),
                }

                false
            },
        );

        let redraw_func = window.make_redraw_notifier();
        let draw_func_weak = Rc::downgrade(&draw_func);
        let progress_cache_weak = Rc::downgrade(&progress_cache);
        gtk::glib::spawn_future_local(async move {
            while let Ok(progress) = r.recv().await {
                if let Some(progress_cache) = progress_cache_weak.upgrade() {
                    progress_cache.set(progress)
                }
                if let Some(draw_func) = Weak::upgrade(&draw_func_weak) {
                    redraw_func(Some(draw_func(progress)))
                }
            }
        });

        struct SlideContext(interval_task::runner::Runner<()>);
        impl Drop for SlideContext {
            fn drop(&mut self) {
                std::mem::take(&mut self.0).close().unwrap();
            }
        }
        window.bind_context(SlideContext(runner));

        let progress = match shell_cmd(&preset_conf.cmd).and_then(|res| {
            use std::str::FromStr;
            f64::from_str(res.trim()).map_err(|_| "Invalid number".to_string())
        }) {
            Ok(p) => p,
            Err(err) => {
                log::error!("slide custom updata error: {err}");
                0.
            }
        };
        progress_cache.set(progress);
        window.redraw(Some(draw_func(progress)));
    } else {
        window.redraw(Some(draw_func(0.)));
    }

    event::setup_event(window, config, &mut w_conf, draw_func, progress_cache);
}
