mod backlight;
mod base;
mod custom;
mod pulseaudio;

use crate::wayland::app::WidgetBuilder;
use config::widgets::slide::base::SlideConfig;

use super::WidgetContext;

pub fn init_widget(
    builder: &mut WidgetBuilder,
    size: (i32, i32),
    w_conf: &mut SlideConfig,
) -> Box<dyn WidgetContext> {
    w_conf.size.calculate_relative(size, w_conf.common.edge);

    use config::widgets::slide::preset::Preset;

    match std::mem::take(&mut w_conf.preset) {
        Preset::Backlight(backlight_config) => {
            Box::new(backlight::preset(builder, w_conf, backlight_config))
        }
        Preset::Speaker(pulse_audio_config) => {
            Box::new(pulseaudio::speaker(builder, w_conf, pulse_audio_config))
        }
        Preset::Microphone(pulse_audio_config) => {
            Box::new(pulseaudio::microphone(builder, w_conf, pulse_audio_config))
        }
        Preset::Custom(custom_config) => {
            Box::new(custom::custom_preset(builder, w_conf, custom_config))
        }
    }
}
