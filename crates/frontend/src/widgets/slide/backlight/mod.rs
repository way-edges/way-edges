use cairo::ImageSurface;
use std::sync::{Arc, Mutex};

use super::base::{
    draw::DrawConfig,
    event::{setup_event, ProgressState},
};
use crate::{
    mouse_state::{MouseEvent, MouseStateData},
    wayland::app::{App, WidgetBuilder},
    window::WidgetContext,
};

use config::{
    widgets::slide::{base::SlideConfig, preset::BacklightConfig},
    Config,
};

pub struct BacklightContext {
    #[allow(dead_code)]
    backend_id: i32,
    device: Option<String>,
    progress: Arc<Mutex<f64>>,

    draw_conf: DrawConfig,

    progress_state: ProgressState,
    only_redraw_on_internal_update: bool,
}
impl WidgetContext for BacklightContext {
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

            backend::backlight::dbus::set_backlight(self.device.as_ref(), p);
        }

        redraw
    }
}

pub fn preset(
    builder: &mut WidgetBuilder,
    conf: &Config,
    mut w_conf: SlideConfig,
    mut preset_conf: BacklightConfig,
) -> impl WidgetContext {
    let device = preset_conf.device.take();
    let progress = Arc::new(Mutex::new(0.));
    let redraw_signal = builder.make_redraw_notifier(None::<fn(&mut App)>);

    let mut backend_cache = 0.;
    let progress_weak = Arc::downgrade(&progress);
    let backend_id = backend::backlight::register_callback(
        move |p| {
            let Some(progress) = progress_weak.upgrade() else {
                return;
            };

            let mut do_redraw = false;
            let mut progress = progress.lock().unwrap();
            if p != *progress {
                *progress = p;
                do_redraw = true
            }
            if p != backend_cache {
                backend_cache = p;
                do_redraw = true
            }
            if do_redraw {
                redraw_signal.ping();
            }
        },
        device.clone(),
    )
    .unwrap();

    BacklightContext {
        backend_id,
        device,
        progress,
        draw_conf: DrawConfig::new(&w_conf, conf.edge),
        progress_state: setup_event(conf, &mut w_conf),
        only_redraw_on_internal_update: w_conf.redraw_only_on_internal_update,
    }
}
