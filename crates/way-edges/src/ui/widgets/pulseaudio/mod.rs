use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{Arc, RwLock},
    time::Duration,
};

use crate::ui::{
    draws::{transition_state::TransitionState, util::color_transition},
    WidgetExposePtr,
};
use backend::pulseaudio::{
    change::{set_mute, set_vol},
    register_callback, unregister_callback, PulseAudioDevice,
};
use config::{
    widgets::pulseaudio::{PAConfig, NAME_SINK, NAME_SOUCE},
    Config,
};
use gtk::{prelude::WidgetExt, ApplicationWindow};

use super::slide;

pub fn init_widget(
    window: &ApplicationWindow,
    config: Config,
    mut pa_conf: PAConfig,
) -> Result<WidgetExposePtr, String> {
    let (debug_widget_name, device) = if pa_conf.is_sink {
        (
            NAME_SINK,
            if let Some(device_desc) = pa_conf.pa_conf.device {
                PulseAudioDevice::NamedSink(device_desc)
            } else {
                PulseAudioDevice::DefaultSink
            },
        )
    } else {
        (
            NAME_SOUCE,
            if let Some(device_desc) = pa_conf.pa_conf.device {
                PulseAudioDevice::NamedSource(device_desc)
            } else {
                PulseAudioDevice::DefaultSource
            },
        )
    };

    let is_mute = Arc::new(RwLock::new(false));
    let mute_transition = Rc::new(RefCell::new(TransitionState::new(Duration::from_millis(
        200,
    ))));
    let exposed = {
        // do not let itself queue_draw, but pulseaudio callback
        let _device = device.clone();
        pa_conf.slide.on_change = Some(Box::new(move |f| {
            set_vol(_device.clone(), f);
            !pa_conf.pa_conf.redraw_only_on_pa_change
        }));

        let _device = device.clone();
        let is_mute_clone = is_mute.clone();
        pa_conf.slide.event_map.as_mut().unwrap().insert(
            3,
            Box::new(move || {
                set_mute(_device.clone(), !*is_mute_clone.read().unwrap());
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
                        mute_transition_clone.borrow().get_y() as f32,
                    ));
                })),
            },
        )?
    };
    let widget_expose = exposed.clone();
    let cb_key = register_callback(
        Box::new(move |vinfo| {
            if let Some(p) = exposed.progress.upgrade() {
                log::debug!("update {debug_widget_name} progress: {vinfo:?}");
                p.set(vinfo.vol);
                if *is_mute.read().unwrap() != vinfo.is_muted {
                    *is_mute.write().unwrap() = vinfo.is_muted;
                    mute_transition
                        .borrow_mut()
                        .set_direction_self(vinfo.is_muted.into());
                }
                if let Some(darea) = exposed.darea.upgrade() {
                    darea.queue_draw();
                }
            }
        }),
        device,
    )?;
    log::debug!("registered pa callback for {debug_widget_name}: {cb_key}");

    window.connect_destroy(move |_| {
        unregister_callback(cb_key);
    });
    Ok(Box::new(widget_expose))
}
