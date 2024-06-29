use crate::config::Config;

use super::draw_area;
use gtk::{prelude::*, Application, CssProvider, STYLE_PROVIDER_PRIORITY_APPLICATION};
use gtk4_layer_shell::LayerShell;

pub fn new_window(app: &Application, mut config: Config) -> gtk::ApplicationWindow {
    let window = gtk::ApplicationWindow::new(app);

    // init layer
    window.init_layer_shell();
    window.set_layer(config.layer);

    // edge and position
    window.set_anchor(config.edge, true);
    if let Some(pos) = config.position {
        window.set_anchor(pos, true);
    }

    // margin
    std::mem::take(&mut config.margins)
        .into_iter()
        .for_each(|(e, m)| {
            window.set_margin(e, m.get_num_into().unwrap());
        });

    draw_area::setup_draw(
        &window,
        config.edge,
        config.get_size_into().unwrap(),
        config.event_map.take().unwrap(),
        config.color,
        config.extra_trigger_size.get_num().unwrap(),
        config.transition_duration,
        config.frame_rate,
    );
    drop(config);

    // set something after show
    window.connect_show(move |w: &gtk::ApplicationWindow| {
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

    window.present();
    window
}
