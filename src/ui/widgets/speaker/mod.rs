use std::{cell::Cell, rc::Weak};

use crate::{
    activate::get_monior_size,
    config::{
        widgets::{slide::SlideConfig, speaker::SpeakerConfig},
        Config,
    },
    plug::pulseaudio::register_callback,
};
use gio::glib::WeakRef;
use gtk::{
    glib,
    prelude::{GtkWindowExt, WidgetExt},
    ApplicationWindow,
};
use libpulse_binding::context::subscribe::InterestMaskSet;

use super::{common, slide};

pub fn init_widget(
    window: &ApplicationWindow,
    config: Config,
    speaker_cfg: SpeakerConfig,
) -> Result<(), String> {
    let exposed = slide::init_widget(window, config, speaker_cfg.slide)?;
    register_callback(
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
    Ok(())
}
