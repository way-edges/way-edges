use crate::{
    config::{widgets::speaker::SpeakerConfig, Config},
    plug::pulseaudio::{register_callback, set_sink_vol, unregister_callback},
};
use gtk::{
    glib,
    prelude::{GtkWindowExt, WidgetExt},
    ApplicationWindow,
};
use libpulse_binding::context::subscribe::InterestMaskSet;

use super::slide;

pub fn init_widget(
    window: &ApplicationWindow,
    config: Config,
    mut speaker_cfg: SpeakerConfig,
) -> Result<(), String> {
    // do not let itself queue_draw, but pulseaudio callback
    speaker_cfg.slide.on_change = Some(Box::new(move |f| {
        set_sink_vol(f);
        !speaker_cfg.speaker.redraw_only_on_pa_change
    }));
    let exposed = slide::init_widget(window, config, speaker_cfg.slide)?;
    let cb_key = register_callback(
        move |vinfo, _| {
            if let Some(p) = exposed.progress.upgrade() {
                log::debug!("update speaker progress: {vinfo:?}");
                p.set(vinfo.vol);
                exposed.darea.upgrade().unwrap().queue_draw();
            }
        },
        Some(glib::clone!(@strong window => move |s| {
            log::error!("Received error from pulseaudio, closing window: {s}");
            window.close();
        })),
        InterestMaskSet::SINK,
    )?;
    log::debug!("register pa callback for speaker: {cb_key}");

    window.connect_destroy(move |_| {
        log::debug!("unregister pa callback for speaker: {cb_key}");
        unregister_callback(cb_key);
    });
    Ok(())
}
