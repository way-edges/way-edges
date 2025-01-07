use cairo::ImageSurface;
use gtk::glib;
use std::{cell::Cell, rc::Rc};

use super::base::{
    draw::DrawConfig,
    event::{setup_event, ProgressState},
};
use crate::{
    mouse_state::{MouseEvent, MouseStateData},
    window::{WidgetContext, WindowContextBuilder},
};

use config::{
    widgets::slide::{base::SlideConfig, preset::BacklightConfig},
    Config,
};

pub struct BacklightContext {
    #[allow(dead_code)]
    backend_id: i32,
    device: Option<String>,
    progress: Rc<Cell<f64>>,

    draw_conf: DrawConfig,

    progress_state: ProgressState,
    only_redraw_on_internal_update: bool,
}
impl WidgetContext for BacklightContext {
    fn redraw(&mut self) -> ImageSurface {
        let p = self.progress.get();
        self.draw_conf.draw(p)
    }

    fn on_mouse_event(&mut self, _: &MouseStateData, event: MouseEvent) -> bool {
        if let Some(p) = self.progress_state.if_change_progress(event.clone()) {
            self.progress.set(p);
            backend::backlight::dbus::set_backlight(self.device.as_ref(), p);
        }

        !self.only_redraw_on_internal_update
    }
}

pub fn preset(
    window: &mut WindowContextBuilder,
    conf: &Config,
    mut w_conf: SlideConfig,
    mut preset_conf: BacklightConfig,
) -> impl WidgetContext {
    let device = preset_conf.device.take();
    let progress = Rc::new(Cell::new(0.));
    let redraw_signal = window.make_redraw_notifier();

    let mut backend_cache = 0.;
    let backend_id = backend::backlight::register_callback(
        glib::clone!(
            #[weak]
            progress,
            move |p| {
                let mut do_redraw = false;
                if p != progress.get() {
                    progress.set(p);
                    do_redraw = true
                }
                if p != backend_cache {
                    backend_cache = p;
                    do_redraw = true
                }
                if do_redraw {
                    redraw_signal()
                }
            }
        ),
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
