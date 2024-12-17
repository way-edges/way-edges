use config::{widgets::backlight::BLConfig, Config};

use crate::ui::WidgetExposePtr;
use backend::backlight::{register_callback, set_backlight, unregister_callback};
use gtk::{prelude::WidgetExt, ApplicationWindow};

use super::slide;

pub fn init_widget(
    window: &ApplicationWindow,
    config: Config,
    mut bl_conf: BLConfig,
) -> Result<WidgetExposePtr, String> {
    let exposed = {
        // do not let itself queue_draw, but pulseaudio callback
        bl_conf.slide.on_change = Some(Box::new(move |f| {
            if let Err(e) = set_backlight(None, f) {
                log::error!("Error setting backlight, closing window: {e}");
            };
            !bl_conf.bl_conf.redraw_only_on_change
        }));

        let add = slide::SlideAdditionalConfig::default(bl_conf.slide.fg_color);
        slide::init_widget_as_plug(window, config, bl_conf.slide, add)?
    };
    let widget_expose = exposed.clone();
    let cb_key = register_callback(
        move |pro| {
            if let Some(p) = exposed.progress.upgrade() {
                log::debug!("update brightness progress: {pro}");
                p.set(pro / 100.);
                if let Some(darea) = exposed.darea.upgrade() {
                    darea.queue_draw();
                }
            }
        },
        bl_conf.bl_conf.device_name,
    )?;
    log::debug!("registered backlight callback for brightness: {cb_key}");

    window.connect_destroy(move |_| {
        unregister_callback(cb_key);
    });
    Ok(Box::new(widget_expose))
}
