use cairo::ImageSurface;
use gtk::{gdk::BUTTON_SECONDARY, glib};
use std::{cell::Cell, rc::Rc, time::Duration};

use super::base::{draw, event};
use crate::window::WindowContext;

use backend::pulseaudio::{
    change::{set_mute, set_vol},
    PulseAudioDevice,
};
use config::{
    widgets::slide::{base::SlideConfig, preset::PulseAudioConfig},
    Config,
};
use util::draw::color_transition;

fn common(
    window: &mut WindowContext,
    config: &Config,
    mut w_conf: SlideConfig,
    preset_conf: PulseAudioConfig,
    device: PulseAudioDevice,
) {
    // TODO: PUT TIME COST INTO CONFIG?
    let mute_animation = window.new_animation(200);
    let non_mute_color = w_conf.fg_color;
    let mute_color = preset_conf.mute_color;
    let progress = Rc::new(Cell::new(0.));
    let (draw_conf, draw_func) = draw::make_draw_func(&w_conf, config.edge);

    window.set_draw_func(Some(glib::clone!(
        #[strong]
        mute_animation,
        #[strong]
        progress,
        move || {
            let mute_y = mute_animation.borrow_mut().progress();
            let fg_color = color_transition(non_mute_color, mute_color, mute_y as f32);
            draw_conf.borrow_mut().fg_color = fg_color;

            let p = progress.get();
            let img = draw_func(p);

            Some(img)
        }
    )));

    let redraw_signal = window.make_redraw_notifier();

    let mute = Rc::new(Cell::new(false));
    let mut backend_mute_cache = 0.;
    let backend_id = backend::pulseaudio::register_callback(
        glib::clone!(
            #[weak]
            progress,
            #[weak]
            mute_animation,
            #[weak]
            mute,
            move |vinfo| {
                let mut do_redraw = false;
                if vinfo.vol != progress.get() {
                    progress.set(vinfo.vol);
                    do_redraw = true
                }
                if vinfo.vol != backend_mute_cache {
                    backend_mute_cache = vinfo.vol;
                    do_redraw = true
                }
                if vinfo.is_muted != mute.get() {
                    mute.set(vinfo.is_muted);
                    mute_animation
                        .borrow_mut()
                        .set_direction(vinfo.is_muted.into());
                    do_redraw = true
                }
                if do_redraw {
                    redraw_signal(None)
                }
            }
        ),
        device.clone(),
    )
    .unwrap();

    // event
    let device_clone = device.clone();
    let key_callback = move |key: u32| {
        if key == BUTTON_SECONDARY {
            set_mute(device_clone.clone(), !mute.get());
        }
    };
    let mut last = None::<Rc<()>>;
    let set_progress_callback = move |p: f64| {
        if let Some(last) = last.take() {
            drop(last)
        }
        let ctx = Rc::new(());
        let device = device.clone();

        // try debouncing
        glib::timeout_add_local_once(
            Duration::from_millis(1),
            glib::clone!(
                #[weak]
                ctx,
                #[weak]
                progress,
                move || {
                    let _ = ctx;
                    progress.set(p);
                    set_vol(device, p);
                }
            ),
        );
        last = Some(ctx)
    };
    event::setup_event(
        window,
        config,
        &mut w_conf,
        Some(key_callback),
        set_progress_callback,
        None::<Rc<fn(f64) -> ImageSurface>>,
    );

    // drop
    struct PABackendContext(i32);
    impl Drop for PABackendContext {
        fn drop(&mut self) {
            backend::pulseaudio::unregister_callback(self.0);
        }
    }
    window.bind_context(PABackendContext(backend_id));
}

pub fn speaker(
    window: &mut WindowContext,
    config: &Config,
    w_conf: SlideConfig,
    mut preset_conf: PulseAudioConfig,
) {
    let device = preset_conf
        .device
        .take()
        .map_or(PulseAudioDevice::DefaultSink, |name| {
            PulseAudioDevice::NamedSink(name)
        });

    common(window, config, w_conf, preset_conf, device);
}

pub fn microphone(
    window: &mut WindowContext,
    config: &Config,
    w_conf: SlideConfig,
    mut preset_conf: PulseAudioConfig,
) {
    let device = preset_conf
        .device
        .take()
        .map_or(PulseAudioDevice::DefaultSource, |name| {
            PulseAudioDevice::NamedSource(name)
        });

    common(window, config, w_conf, preset_conf, device);
}
