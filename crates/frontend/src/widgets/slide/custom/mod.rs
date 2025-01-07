use cairo::ImageSurface;
use interval_task::runner::Runner;
use std::{cell::Cell, rc::Rc, time::Duration};

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
    window::{WidgetContext, WindowContextBuilder},
};

pub struct CustomContext {
    #[allow(dead_code)]
    runner: Option<Runner<()>>,
    progress: Rc<Cell<f64>>,
    event_map: KeyEventMap,
    on_change: Option<Template>,

    draw_conf: DrawConfig,

    progress_state: ProgressState,
    only_redraw_on_internal_update: bool,
}
impl WidgetContext for CustomContext {
    fn redraw(&mut self) -> ImageSurface {
        let p = self.progress.get();
        self.draw_conf.draw(p)
    }

    fn on_mouse_event(&mut self, _: &MouseStateData, event: MouseEvent) -> bool {
        if let Some(p) = self.progress_state.if_change_progress(event.clone()) {
            self.progress.set(p);

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

        !self.only_redraw_on_internal_update
    }
}

pub fn custom_preset(
    window: &mut WindowContextBuilder,
    conf: &Config,
    mut w_conf: SlideConfig,
    mut preset_conf: CustomConfig,
) -> impl WidgetContext {
    let progress = Rc::new(Cell::new(0.));

    // interval
    let runner = interval_update(window, &preset_conf, &progress);

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
    window: &mut WindowContextBuilder,
    preset_conf: &CustomConfig,
    progress_cache: &Rc<Cell<f64>>,
) -> Option<Runner<()>> {
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

        let redraw_signal = window.make_redraw_notifier();
        let progress_cache_weak = Rc::downgrade(progress_cache);
        gtk::glib::spawn_future_local(async move {
            while let Ok(progress) = r.recv().await {
                if let Some(progress_cache) = progress_cache_weak.upgrade() {
                    progress_cache.set(progress)
                }
                redraw_signal()
            }
        });
        Some(runner)
    } else {
        None
    }
}
