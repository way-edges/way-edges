pub mod event;

use std::{
    cell::Cell,
    rc::{Rc, Weak},
    time::Duration,
};

use crate::window::WindowContext;
use backend::pulseaudio::PulseAudioDevice;
use cairo::ImageSurface;
use config::{
    widgets::slide::{base::SlideConfig, preset::PulseAudioConfig},
    Config,
};

pub fn speaker(
    window: &mut WindowContext,
    config: &Config,
    mut w_conf: SlideConfig,
    preset_conf: PulseAudioConfig,
) {
    let device = preset_conf
        .device
        .map_or(PulseAudioDevice::DefaultSink, |name| {
            PulseAudioDevice::NamedSink(name)
        });

    // TODO: MAKE TIME COST INTO CONFIG?
    let mut_animation = window.new_animation(200);

    // NOTE: THIS TYPE ANNOTATION IS WEIRD
    window.set_draw_func(None::<fn() -> Option<ImageSurface>>);

    let (_, draw_func) = super::base::draw::make_draw_func(&w_conf, config.edge);
    let draw_func = Rc::new(draw_func);
    let progress_cache = Rc::new(Cell::new(0.));

    event::setup_event(window, config, &mut w_conf, draw_func, progress_cache);
}
