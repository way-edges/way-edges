use std::{
    cell::Cell,
    rc::Rc,
    sync::{Arc, RwLock},
    time::Duration,
};

use crate::{
    config::{
        widgets::pulseaudio::{PAConfig, NAME_SINK, NAME_SOUCE},
        Config,
    },
    plug::pulseaudio::{
        register_callback, set_sink_mute, set_sink_vol, set_source_mute, set_source_vol,
        unregister_callback,
    },
    ui::draws::transition_state::TransitionState,
};
use gtk::{
    gdk::RGBA,
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

    let is_mute = Arc::new(RwLock::new(false));
    let mute_transition = TransitionState::<f64>::new(Duration::from_millis(200), (0.0, 1.0));
    let exposed = {
        // do not let itself queue_draw, but pulseaudio callback
        pa_conf.slide.on_change = Some(Box::new(move |f| {
            on_change_func(f);
            !pa_conf.pa_conf.redraw_only_on_pa_change
        }));
        let is_mute_clone = is_mute.clone();
        pa_conf.slide.event_map.as_mut().unwrap().insert(
            3,
            Box::new(move || {
                mute_func(!*is_mute_clone.read().unwrap());
            }),
        );

        let (start_color, stop_color) = (pa_conf.slide.fg_color, pa_conf.pa_conf.mute_color);
        let mute_color = Rc::new(Cell::new(start_color));
        let mute_transition_clone = mute_transition.clone();
        slide::init_widget_as_plug(
            window,
            config,
            pa_conf.slide,
            slide::SlideAdditionalConfig {
                fg_color: mute_color.clone(),
                additional_transitions: vec![mute_transition.clone()],
                on_draw: Some(Box::new(move || {
                    // calculate color
                    mute_color.set(color_transition(
                        start_color,
                        stop_color,
                        mute_transition_clone.get_y() as f32,
                    ));
                })),
            },
        )?
    };
    let cb_key = register_callback(
        move |vinfo, _| {
            if let Some(p) = exposed.progress.upgrade() {
                log::debug!("update {debug_name} progress: {vinfo:?}");
                p.set(vinfo.vol);
                if *is_mute.read().unwrap() != vinfo.is_muted {
                    *is_mute.write().unwrap() = vinfo.is_muted;
                    mute_transition.set_direction_self(vinfo.is_muted);
                }
                exposed.darea.upgrade().unwrap().queue_draw();
            }
        },
        Some(glib::clone!(@strong window => move |s| {
            log::error!("Received error from pulseaudio, closing window: {s}");
            window.close();
        })),
        maskset,
    )?;
    log::debug!("registered pa callback for {debug_name}: {cb_key}");

    window.connect_destroy(move |_| {
        log::debug!("unregister pa callback for {debug_name}: {cb_key}");
        unregister_callback(cb_key);
    });
    Ok(())
}

fn color_transition(start_color: RGBA, stop_color: RGBA, v: f32) -> RGBA {
    let r = start_color.red() + (stop_color.red() - start_color.red()) * v;
    let g = start_color.green() + (stop_color.green() - start_color.green()) * v;
    let b = start_color.blue() + (stop_color.blue() - start_color.blue()) * v;
    let a = start_color.alpha() + (stop_color.alpha() - start_color.alpha()) * v;
    RGBA::new(r, g, b, a)
}
