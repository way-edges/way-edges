mod backlight;
mod base;
mod custom;
mod pulseaudio;

use crate::{wayland::app::WidgetBuilder, window::WidgetContext};
use config::{widgets::slide::base::SlideConfig, Config};

pub fn init_widget(
    builder: &mut WidgetBuilder,
    size: (i32, i32),
    config: &Config,
    mut w_conf: SlideConfig,
) -> Box<dyn WidgetContext> {
    w_conf.size.calculate_relative(size, config.edge);

    use config::widgets::slide::preset::Preset;

    match std::mem::take(&mut w_conf.preset) {
        Preset::Backlight(backlight_config) => {
            Box::new(backlight::preset(builder, config, w_conf, backlight_config))
        }
        Preset::Speaker(pulse_audio_config) => Box::new(pulseaudio::speaker(
            builder,
            config,
            w_conf,
            pulse_audio_config,
        )),
        Preset::Microphone(pulse_audio_config) => Box::new(pulseaudio::microphone(
            builder,
            config,
            w_conf,
            pulse_audio_config,
        )),
        Preset::Custom(custom_config) => Box::new(custom::custom_preset(
            builder,
            config,
            w_conf,
            custom_config,
        )),
    }
}
