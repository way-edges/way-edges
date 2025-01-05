mod draw;

use crate::{
    mouse_state::{MouseEvent, MouseStateData},
    window::{WidgetContext, WindowContextBuilder},
};
use config::{
    widgets::{button::BtnConfig, common::KeyEventMap},
    Config,
};
use draw::DrawConfig;
use gtk::{gdk::Monitor, prelude::MonitorExt};

pub fn init_widget(
    _: &mut WindowContextBuilder,
    monitor: &Monitor,
    config: Config,
    mut btn_config: BtnConfig,
) -> impl WidgetContext {
    let geom = monitor.geometry();
    let size = (geom.width(), geom.height());
    btn_config.size.calculate_relative(size, config.edge);

    BtnContext {
        draw_conf: DrawConfig::new(&btn_config, config.edge),
        pressing: false,
        event_map: std::mem::take(&mut btn_config.event_map),
    }
}

pub struct BtnContext {
    draw_conf: DrawConfig,
    pressing: bool,
    event_map: KeyEventMap,
}
impl WidgetContext for BtnContext {
    fn redraw(&mut self) -> cairo::ImageSurface {
        self.draw_conf.draw(self.pressing)
    }

    fn on_mouse_event(&mut self, data: &MouseStateData, event: MouseEvent) -> bool {
        if let MouseEvent::Release(_, k) = event {
            self.event_map.call(k);
        }

        let new_pressing_state = data.pressing.is_some();
        if new_pressing_state != self.pressing {
            self.pressing = new_pressing_state;
            true
        } else {
            false
        }
    }
}
