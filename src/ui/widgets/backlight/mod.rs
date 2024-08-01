use crate::{
    config::{widgets::backlight::BLConfig, Config},
    plug::backlight::{register_callback, set_backlight, unregister_callback},
    ui::{draws::mouse_state::TranslateStateExpose, WidgetExposePtr},
};
use gtk::{
    glib,
    prelude::{GtkWindowExt, WidgetExt},
    ApplicationWindow,
};

use super::slide;

pub fn init_widget(
    window: &ApplicationWindow,
    config: Config,
    mut bl_conf: BLConfig,
) -> Result<WidgetExposePtr, String> {
    let exposed = {
        // do not let itself queue_draw, but pulseaudio callback
        let (s, r) = async_channel::bounded(1);
        bl_conf.slide.on_change = Some(Box::new(glib::clone!(
            #[strong]
            s,
            move |f| {
                if let Err(e) = set_backlight(None, f) {
                    log::error!("Error setting backlight, closing window: {e}");
                    s.try_send(()).ok();
                };
                !bl_conf.bl_conf.redraw_only_on_change
            }
        )));

        let add = slide::SlideAdditionalConfig::default(bl_conf.slide.fg_color);
        let exposed = slide::init_widget_as_plug(window, config, bl_conf.slide, add)?;
        glib::spawn_future_local(glib::clone!(
            #[weak]
            window,
            async move {
                if r.recv().await.is_ok() {
                    window.close()
                }
            }
        ));
        exposed
    };
    let bl_expose = TranslateStateExpose::new(exposed.tls.clone());
    let cb_key = register_callback(
        move |pro| {
            if let Some(p) = exposed.progress.upgrade() {
                log::debug!("update brightness progress: {pro}");
                p.set(pro / 100.);
                exposed.darea.upgrade().unwrap().queue_draw();
            }
        },
        Some(glib::clone!(
            #[strong]
            window,
            move |s| {
                log::error!("Received error from backlight, closing window: {s}");
                window.close();
            }
        )),
        bl_conf.bl_conf.device_name,
    )?;
    log::debug!("registered pa callback for brightness: {cb_key}");

    window.connect_destroy(move |_| {
        log::debug!("unregister pa callback for brightness: {cb_key}");
        unregister_callback(cb_key);
    });
    Ok(Box::new(bl_expose))
}
