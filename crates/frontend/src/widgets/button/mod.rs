mod draw;

use crate::{
    mouse_state::{MouseEvent, MouseStateData},
    wayland::app::WidgetBuilder,
};
use config::{shared::KeyEventMap, widgets::button::BtnConfig};
use draw::DrawConfig;

use super::WidgetContext;

pub fn init_widget(
    builder: &mut WidgetBuilder,
    size: (i32, i32),
    mut btn_config: BtnConfig,
) -> impl WidgetContext {
    let edge = builder.common_config.edge;
    btn_config.size.calculate_relative(size, edge);

    BtnContext {
        draw_conf: DrawConfig::new(&btn_config, edge),
        pressing: false,
        event_map: btn_config.event_map,
    }
}

#[derive(Debug)]
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
