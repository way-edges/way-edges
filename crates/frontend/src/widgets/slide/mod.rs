mod base;
mod custom;
mod pulseaudio;

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

    use config::widgets::slide::preset::Preset;

    match std::mem::take(&mut w_conf.preset) {
        Preset::Backlight(backlight_config) => todo!(),
        Preset::Speaker(pulse_audio_config) => {
            pulseaudio::speaker(window, &config, w_conf, pulse_audio_config)
        }
        Preset::Microphone(pulse_audio_config) => {
            pulseaudio::microphone(window, &config, w_conf, pulse_audio_config)
        }
        Preset::Custom(custom_config) => {
            custom::custom_preset(window, &config, w_conf, custom_config)
        }
    }
}
