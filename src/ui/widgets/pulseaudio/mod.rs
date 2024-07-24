use std::{
    cell::{Cell, RefCell},
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
        register_callback, set_mute, set_vol, unregister_callback, OptionalSinkOrSource,
    },
    ui::draws::transition_state::TransitionState,
};
use gtk::{gdk::RGBA, prelude::WidgetExt, ApplicationWindow};

use super::slide;

pub fn init_widget(
    window: &ApplicationWindow,
    config: Config,
    mut pa_conf: PAConfig,
) -> Result<(), String> {
    let (debug_name, sos) = match pa_conf.is_sink {
        true => (
            NAME_SINK,
            OptionalSinkOrSource::sink(pa_conf.pa_conf.device),
        ),
        false => (
            NAME_SOUCE,
            OptionalSinkOrSource::source(pa_conf.pa_conf.device),
        ),
    };

    let is_mute = Arc::new(RwLock::new(false));
    let mute_transition = Rc::new(RefCell::new(TransitionState::new(Duration::from_millis(
        200,
    ))));
    let exposed = {
        // do not let itself queue_draw, but pulseaudio callback
        let _sos = sos.clone();
        pa_conf.slide.on_change = Some(Box::new(move |f| {
            set_vol(_sos.clone(), f);
            !pa_conf.pa_conf.redraw_only_on_pa_change
        }));

        let _sos = sos.clone();
        let is_mute_clone = is_mute.clone();
        pa_conf.slide.event_map.as_mut().unwrap().insert(
            3,
            Box::new(move || {
                set_mute(_sos.clone(), !*is_mute_clone.read().unwrap());
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
    let cb_key = register_callback(
        Box::new(move |vinfo| {
            if let Some(p) = exposed.progress.upgrade() {
                log::debug!("update {debug_name} progress: {vinfo:?}");
                p.set(vinfo.vol);
                if *is_mute.read().unwrap() != vinfo.is_muted {
                    *is_mute.write().unwrap() = vinfo.is_muted;
                    mute_transition
                        .borrow_mut()
                        .set_direction_self(vinfo.is_muted.into());
                }
                exposed.darea.upgrade().unwrap().queue_draw();
            }
        }),
        // Some(glib::clone!(#[strong] window , move |s| {
        //     log::error!("Received error from pulseaudio, closing window: {s}");
        //     window.close();
        // })),
        sos,
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
