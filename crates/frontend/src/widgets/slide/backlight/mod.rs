use cairo::ImageSurface;
use gtk::glib;
use std::{cell::Cell, rc::Rc};

use super::base::{draw, event};
use crate::window::WindowContext;

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
    // NOTE: THIS TYPE ANNOTATION IS WEIRD
    window.set_draw_func(None::<fn() -> Option<ImageSurface>>);

    let device = preset_conf.device.take();

    let progress = Rc::new(Cell::new(0.));
    let redraw_signal = window.make_redraw_notifier();

    let (_, draw_func) = draw::make_draw_func(&w_conf, config.edge);

    let draw_func = Rc::new(draw_func);

    let mut backend_cache = 0.;
    let backend_id = backend::backlight::register_callback(
        glib::clone!(
            #[weak]
            progress,
            #[weak]
            draw_func,
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
                    redraw_signal(Some(draw_func(p)))
                }
            }
        ),
        device.clone(),
    )
    .unwrap();

    // event
    let set_progress_callback = move |p: f64| {
        progress.set(p);
        let device = device.clone();
        backend::backlight::dbus::set_backlight(device, p);
    };
    event::setup_event(
        window,
        config,
        &mut w_conf,
        None::<fn(u32)>,
        set_progress_callback,
        Some(draw_func),
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
