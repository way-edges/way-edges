use crate::config::{self, Config};

use gio::glib::WeakRef;
use gtk::{
    gdk::Monitor, prelude::*, Application, ApplicationWindow, CssProvider,
    STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use gtk4_layer_shell::LayerShell;

use super::widgets;

pub type WidgetExposePtr = Box<dyn WidgetExpose>;
pub trait WidgetExpose {
    fn close(&mut self) {}
    fn toggle_pin(&mut self) {}
}

pub struct WidgetCtx {
    pub window: WeakRef<ApplicationWindow>,
    pub widget_expose: WidgetExposePtr,
}
impl WidgetCtx {
    pub fn close(&mut self) {
        if let Some(w) = self.window.upgrade() {
            w.close()
        }
        self.widget_expose.close()
    }
}
impl Drop for WidgetCtx {
    fn drop(&mut self) {
        self.close()
    }
}

pub fn new_window(
    app: &Application,
    mut config: Config,
    monitor: &Monitor,
) -> Result<WidgetCtx, String> {
    let window = gtk::ApplicationWindow::new(app);
    println!("ONCE!!!");

    // init layer
    window.init_layer_shell();
    window.set_monitor(monitor);
    window.set_layer(config.layer);

    // edge and position
    window.set_anchor(config.edge, true);
    window.set_anchor(config.position, true);

    // set something after show
    window.connect_show(|w: &gtk::ApplicationWindow| {
        // transparency background !! may not work for some gtk4 theme, and idk what to do with it !!
        let provider = CssProvider::new();
        provider
            // .load_from_string("window.background { background: unset; border: 1px solid white; }");
            .load_from_string("window.background { background: unset; }");
        gtk::style_context_add_provider_for_display(
            &WidgetExt::display(w),
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    // margin
    std::mem::take(&mut config.margins)
        .into_iter()
        .try_for_each(|(e, m)| -> Result<(), String> {
            window.set_margin(e, m.get_num_into()? as i32);
            Ok(())
        })?;

    // widget
    let widget = match config.widget.take().ok_or("Widget is None")? {
        config::Widget::Btn(c) => widgets::button::init_widget(&window, config, *c),
        config::Widget::Slider(c) => widgets::slide::init_widget(&window, config, *c),
        config::Widget::PulseAudio(c) => widgets::pulseaudio::init_widget(&window, config, *c),
        config::Widget::Backlight(c) => widgets::backlight::init_widget(&window, config, *c),
        config::Widget::WrapBox(c) => widgets::wrapbox::init_widget(&window, config, *c),
        config::Widget::HyprWorkspace(c) => {
            widgets::hypr_workspace::init_widget(&window, config, *c)
        }
        _ => return Err("Unsupported window widget".to_string()),
    }?;

    window.connect_destroy(|_| {
        log::info!("destroy window");
    });

    Ok(WidgetCtx {
        window: window.downgrade(),
        widget_expose: widget,
    })
}
