mod base;

use crate::window::WindowContext;
use config::{widgets::slide::base::SlideConfig, Config};
use gtk::{gdk::Monitor, prelude::MonitorExt};

pub fn init_widget(
    window: &mut WindowContext,
    monitor: &Monitor,
    config: Config,
    mut w_conf: SlideConfig,
) {
    let geom = monitor.geometry();
    let size = (geom.width(), geom.height());
    w_conf.size.calculate_relative(size, config.edge);

    event::setup_event(window, &config, &mut btn_config);
}
