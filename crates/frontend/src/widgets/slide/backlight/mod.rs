use cairo::ImageSurface;
use gtk::glib;
use std::{cell::Cell, rc::Rc};

use super::base::event;
use crate::window::WindowContext;

use backend::backlight::set_backlight;
use config::{
    widgets::slide::{base::SlideConfig, preset::BacklightConfig},
    Config,
};

pub fn preset(
    window: &mut WindowContext,
    config: &Config,
    mut w_conf: SlideConfig,
    mut preset_conf: BacklightConfig,
) {
    let device = preset_conf.device.take();

    let progress = Rc::new(Cell::new(0.));
    let redraw_signal = window.make_redraw_notifier();

    let mut backend_cache = 0.;
    let backend_id = backend::backlight::register_callback(
        glib::clone!(
            #[weak]
            progress,
            move |p| {
                let mut do_redraw = false;
                if p != progress.get() {
                    progress.set(p);
                    do_redraw = true
                }
                if p != backend_cache {
                    backend_cache = p;
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
    let set_progress_callback = move |p: f64| {
        let device = device.clone();
        set_backlight(device, p).unwrap();
    };
    event::setup_event(
        window,
        config,
        &mut w_conf,
        None::<fn(u32)>,
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
