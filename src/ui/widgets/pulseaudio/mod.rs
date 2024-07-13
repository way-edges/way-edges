use crate::{
    config::{
        widgets::pulseaudio::{PAConfig, NAME_SINK, NAME_SOUCE},
        Config,
    },
    plug::pulseaudio::{
        register_callback, set_sink_mute, set_sink_vol, set_source_mute, set_source_vol,
        unregister_callback,
    },
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
    mut pa_conf: PAConfig,
) -> Result<(), String> {
    type OnChangeFunc = Box<dyn Fn(f64) + 'static + Send + Sync>;
    type OnMuteFunc = Box<dyn Fn(bool) + 'static + Send + Sync>;
    let (debug_name, maskset, on_change_func, mute_func) = match pa_conf.is_sink {
        true => (
            NAME_SINK,
            InterestMaskSet::SINK,
            Box::new(set_sink_vol) as OnChangeFunc,
            Box::new(set_sink_mute) as OnMuteFunc,
        ),
        false => (
            NAME_SOUCE,
            InterestMaskSet::SOURCE,
            Box::new(set_source_vol) as OnChangeFunc,
            Box::new(set_source_mute) as OnMuteFunc,
        ),
    };

    // do not let itself queue_draw, but pulseaudio callback
    pa_conf.slide.on_change = Some(Box::new(move |f| {
        on_change_func(f);
        !pa_conf.pa_conf.redraw_only_on_pa_change
    }));
    let exposed = slide::init_widget(window, config, pa_conf.slide)?;
    let cb_key = register_callback(
        move |vinfo, _| {
            if let Some(p) = exposed.progress.upgrade() {
                log::debug!("update {debug_name} progress: {vinfo:?}");
                p.set(vinfo.vol);
                exposed.darea.upgrade().unwrap().queue_draw();
            }
        },
        Some(glib::clone!(@strong window => move |s| {
            log::error!("Received error from pulseaudio, closing window: {s}");
            window.close();
        })),
        maskset,
    )?;
    log::debug!("register pa callback for {debug_name}: {cb_key}");

    window.connect_destroy(move |_| {
        log::debug!("unregister pa callback for {debug_name}: {cb_key}");
        unregister_callback(cb_key);
    });
    Ok(())
}
