mod backlight;
mod base;
mod custom;
mod pulseaudio;

use std::{cell::RefCell, rc::Rc};

use crate::window::{WidgetContext, WindowContextBuilder};
use config::{widgets::slide::base::SlideConfig, Config};
use gtk::{gdk::Monitor, prelude::MonitorExt};

pub fn init_widget(
    window: &mut WindowContextBuilder,
    monitor: &Monitor,
    config: Config,
    mut w_conf: SlideConfig,
) -> Rc<RefCell<dyn WidgetContext>> {
    let geom = monitor.geometry();
    let size = (geom.width(), geom.height());
    w_conf.size.calculate_relative(size, config.edge);

    use config::widgets::slide::preset::Preset;

    match std::mem::take(&mut w_conf.preset) {
        Preset::Backlight(backlight_config) => {
            backlight::preset(window, &config, w_conf, backlight_config).make_rc()
        }
        Preset::Speaker(pulse_audio_config) => {
            pulseaudio::speaker(window, &config, w_conf, pulse_audio_config).make_rc()
        }
        Preset::Microphone(pulse_audio_config) => {
            pulseaudio::microphone(window, &config, w_conf, pulse_audio_config).make_rc()
        }
        Preset::Custom(custom_config) => {
            custom::custom_preset(window, &config, w_conf, custom_config).make_rc()
        }
    }
}
