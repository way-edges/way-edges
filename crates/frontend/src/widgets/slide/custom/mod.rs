use cairo::ImageSurface;
use interval_task::runner::Runner;
use std::{cell::Cell, rc::Rc, time::Duration};

use config::{
    shared::KeyEventMap,
    widgets::slide::{base::SlideConfig, preset::CustomConfig},
};
use util::{
    shell::{shell_cmd, shell_cmd_non_block},
    template::base::Template,
};

use super::base::{
    draw::DrawConfig,
    event::{setup_event, ProgressState},
};
use crate::{
    mouse_state::{MouseEvent, MouseStateData},
    wayland::app::WidgetBuilder,
    widgets::WidgetContext,
};

#[derive(Debug)]
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
            if !self.only_redraw_on_internal_update {
                self.progress.set(p);
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

        !self.only_redraw_on_internal_update
    }
}

pub fn custom_preset(
    builder: &mut WidgetBuilder,
    w_conf: SlideConfig,
    mut preset_conf: CustomConfig,
) -> impl WidgetContext {
    let progress = Rc::new(Cell::new(0.));

    // interval
    let runner = interval_update(builder, &preset_conf, &progress);

    // key event map
    let event_map = std::mem::take(&mut preset_conf.event_map);

    // on change
    let on_change = preset_conf.on_change_command.take();

    let edge = builder.common_config.edge;
    CustomContext {
        runner,
        progress,
        event_map,
        on_change,
        draw_conf: DrawConfig::new(edge, &w_conf),
        progress_state: setup_event(edge, &w_conf),
        only_redraw_on_internal_update: w_conf.redraw_only_on_internal_update,
    }
}

fn interval_update(
    window: &mut WidgetBuilder,
    preset_conf: &CustomConfig,
    progress_cache: &Rc<Cell<f64>>,
) -> Option<Runner<()>> {
    if preset_conf.update_interval > 0 && !preset_conf.update_command.is_empty() {
        let progress_cache_weak = Rc::downgrade(progress_cache);
        let redraw_signal = window.make_redraw_channel(move |_, p| {
            let Some(progress_cache) = progress_cache_weak.upgrade() else {
                return;
            };
            progress_cache.set(p);
        });

        let cmd = preset_conf.update_command.clone();
        let mut runner = interval_task::runner::new_runner(
            Duration::from_millis(preset_conf.update_interval),
            || (),
            move |_| {
                match shell_cmd(&cmd).and_then(|res| {
                    use std::str::FromStr;
                    f64::from_str(res.trim()).map_err(|_| "Invalid number".to_string())
                }) {
                    Ok(progress) => {
                        redraw_signal.send(progress).unwrap();
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
