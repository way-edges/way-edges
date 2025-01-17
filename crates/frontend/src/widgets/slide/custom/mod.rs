use cairo::ImageSurface;
use interval_task::runner::Runner;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use config::{
    widgets::{
        common::KeyEventMap,
        slide::{base::SlideConfig, preset::CustomConfig},
    },
    Config,
};
use util::{
    shell::{shell_cmd, shell_cmd_non_block},
    template::base::Template,
};

use super::base::{
    draw::{self, DrawConfig},
    event::{setup_event, ProgressState},
};
use crate::{
    mouse_state::{MouseEvent, MouseStateData},
    wayland::app::{App, WidgetBuilder},
    window::WidgetContext,
};

pub struct CustomContext {
    #[allow(dead_code)]
    runner: Option<Runner<()>>,
    progress: Arc<Mutex<f64>>,
    event_map: KeyEventMap,
    on_change: Option<Template>,

    draw_conf: DrawConfig,

    progress_state: ProgressState,
    only_redraw_on_internal_update: bool,
}
impl WidgetContext for CustomContext {
    fn redraw(&mut self) -> ImageSurface {
        let p = *self.progress.lock().unwrap();
        self.draw_conf.draw(p)
    }

    fn on_mouse_event(&mut self, _: &MouseStateData, event: MouseEvent) -> bool {
        let mut redraw = false;

        if let Some(p) = self.progress_state.if_change_progress(event.clone()) {
            if !self.only_redraw_on_internal_update {
                let mut old_p = self.progress.lock().unwrap();
                if *old_p != p {
                    *old_p = p;
                    redraw = true
                }
            }

            if let Some(template) = self.on_change.as_mut() {
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
        }

        if let MouseEvent::Release(_, key) = event {
            self.event_map.call(key);
        }

        redraw
    }
}

pub fn custom_preset(
    builder: &mut WidgetBuilder,
    conf: &Config,
    mut w_conf: SlideConfig,
    mut preset_conf: CustomConfig,
) -> impl WidgetContext {
    let progress = Arc::new(Mutex::new(0.));

    // interval
    let runner = interval_update(builder, &preset_conf, &progress);

    // key event map
    let event_map = std::mem::take(&mut preset_conf.event_map);

    // on change
    let on_change = preset_conf.on_change.take();

    CustomContext {
        runner,
        progress,
        event_map,
        on_change,
        draw_conf: draw::DrawConfig::new(&w_conf, conf.edge),
        progress_state: setup_event(conf, &mut w_conf),
        only_redraw_on_internal_update: w_conf.redraw_only_on_internal_update,
    }
}

fn interval_update(
    window: &mut WidgetBuilder,
    preset_conf: &CustomConfig,
    progress_cache: &Arc<Mutex<f64>>,
) -> Option<Runner<()>> {
    if preset_conf.interval_update.0 > 0 && !preset_conf.interval_update.1.is_empty() {
        let redraw_signal = window.make_redraw_notifier(None::<fn(&mut App)>);
        let progress_cache_weak = Arc::downgrade(progress_cache);

        let cmd = preset_conf.interval_update.1.clone();
        let mut runner = interval_task::runner::new_runner(
            Duration::from_millis(preset_conf.interval_update.0),
            || (),
            move |_| {
                let Some(progress_cache) = progress_cache_weak.upgrade() else {
                    return true;
                };

                match shell_cmd(&cmd).and_then(|res| {
                    use std::str::FromStr;
                    f64::from_str(res.trim()).map_err(|_| "Invalid number".to_string())
                }) {
                    Ok(progress) => {
                        *progress_cache.lock().unwrap() = progress;
                        redraw_signal.ping();
                    }
                    Err(err) => log::error!("slide custom updata error: {err}"),
                }

                false
            },
        );
        runner.start().unwrap();

        Some(runner)
    } else {
        None
    }
}
