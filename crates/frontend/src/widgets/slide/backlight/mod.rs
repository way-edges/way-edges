use cairo::ImageSurface;
use std::{cell::Cell, rc::Rc};

use super::base::{
    draw::DrawConfig,
    event::{setup_event, ProgressState},
};
use crate::{
    mouse_state::{MouseEvent, MouseStateData},
    wayland::app::WidgetBuilder,
    widgets::WidgetContext,
};

use config::widgets::slide::{base::SlideConfig, preset::BacklightConfig};

#[derive(Debug)]
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
            if !self.only_redraw_on_internal_update {
                self.progress.set(p);
            }

            backend::backlight::dbus::set_backlight(self.device.as_ref(), p);
        }

        !self.only_redraw_on_internal_update
    }
}

pub fn preset(
    builder: &mut WidgetBuilder,
    w_conf: SlideConfig,
    mut preset_conf: BacklightConfig,
) -> impl WidgetContext {
    let device = preset_conf.device.take();
    let progress = Rc::new(Cell::new(0.));

    let progress_weak = Rc::downgrade(&progress);
    let redraw_signal = builder.make_redraw_channel(move |_, p| {
        let Some(progress) = progress_weak.upgrade() else {
            return;
        };
        progress.set(p);
    });
    let backend_id = backend::backlight::register_callback(redraw_signal, device.clone()).unwrap();

    let edge = builder.common_config.edge;
    BacklightContext {
        backend_id,
        device,
        progress,
        draw_conf: DrawConfig::new(edge, &w_conf),
        progress_state: setup_event(edge, &w_conf),
        only_redraw_on_internal_update: w_conf.redraw_only_on_internal_update,
    }
}
