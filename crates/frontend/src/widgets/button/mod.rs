mod draw;
mod event;

use crate::window::WindowContext;
use config::{widgets::button::BtnConfig, Config};
use gtk::{gdk::Monitor, prelude::MonitorExt};

pub fn init_widget(
    window: &mut WindowContext,
    monitor: &Monitor,
    config: Config,
    mut btn_config: BtnConfig,
) {
    let geom = monitor.geometry();
    let size = (geom.width(), geom.height());
    btn_config.size.calculate_relative(size, config.edge);

    event::setup_event(window, &config, &mut btn_config);
}
