use cairo::ImageSurface;
use std::{
    cell::Cell,
    rc::{Rc, Weak},
    time::Duration,
};

use config::{
    widgets::slide::{base::SlideConfig, preset::CustomConfig},
    Config,
};
use util::shell::{shell_cmd, shell_cmd_non_block};

use super::base::{draw, event};
use crate::window::WindowContext;

pub fn custom_preset(
    window: &mut WindowContext,
    config: &Config,
    mut w_conf: SlideConfig,
    mut preset_conf: CustomConfig,
) {
    // NOTE: THIS TYPE ANNOTATION IS WEIRD
    window.set_draw_func(None::<fn() -> Option<ImageSurface>>);

    let (_, draw_func) = draw::make_draw_func(&w_conf, config.edge);
    let draw_func = Rc::new(draw_func);
    let progress_cache = Rc::new(Cell::new(0.));

    // interval
    interval_update(window, &preset_conf, &progress_cache, &draw_func);

    // key event map
    let key_map = std::mem::take(&mut preset_conf.event_map);
    let key_callback = move |k: u32| {
        key_map.call(k);
    };

    // on change
    let mut on_change = preset_conf.on_change.take();
    let set_progress_callback = move |p: f64| {
        progress_cache.set(p);
        if let Some(template) = on_change.as_mut() {
            use util::template::arg;
            let cmd = template.parse(|parser| {
                let res = match parser.name() {
                    arg::TEMPLATE_ARG_FLOAT => {
                        let float_parser = parser
                            .downcast_ref::<util::template::arg::TemplateArgFloatParser>()
                            .unwrap();
                        float_parser.parse(p).clone()
                    }
                    _ => unreachable!(),
                };
                res
            });
            shell_cmd_non_block(cmd);
        }
    };

    event::setup_event(
        window,
        config,
        &mut w_conf,
        Some(key_callback),
        set_progress_callback,
        Some(draw_func),
    );
}

fn interval_update(
    window: &mut WindowContext,
    preset_conf: &CustomConfig,

    progress_cache: &Rc<Cell<f64>>,
    draw_func: &Rc<impl 'static + Fn(f64) -> ImageSurface>,
) {
    if preset_conf.interval_update.0 > 0 && !preset_conf.interval_update.1.is_empty() {
        let (s, r) = async_channel::bounded(1);
        let cmd = preset_conf.interval_update.1.clone();
        let mut runner = interval_task::runner::new_runner(
            Duration::from_millis(preset_conf.interval_update.0),
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
        runner.start().unwrap();

        let redraw_func = window.make_redraw_notifier();
        let draw_func_weak = Rc::downgrade(draw_func);
        let progress_cache_weak = Rc::downgrade(progress_cache);
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

        // bind runner to window
        // ensure runner is closed when window destroyed
        struct SlideContext(interval_task::runner::Runner<()>);
        impl Drop for SlideContext {
            fn drop(&mut self) {
                std::mem::take(&mut self.0).close().unwrap();
            }
        }
        window.bind_context(SlideContext(runner));

        // initial progress
        let progress = match shell_cmd(&preset_conf.interval_update.1).and_then(|res| {
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
}
