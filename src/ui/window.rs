use super::draw_area;
use gtk::{
    cairo::{RectangleInt, Region},
    prelude::*,
    Application, CssProvider, STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use gtk4_layer_shell::{Edge, Layer, LayerShell};

pub fn new_window(app: &Application, size: (f64, f64)) {
    // Create a normal GTK window however you like
    let window = gtk::ApplicationWindow::new(app);
    window.connect_show(move |w: &gtk::ApplicationWindow| {
        // set input region
        w.surface()
            .expect("Surface not detected")
            .set_input_region(&Region::create_rectangle(&RectangleInt::new(
                0,
                0,
                size.0 as i32,
                size.1 as i32,
            )));

        // transparency background !! may not work for some gtk4 theme, and idk how to fix !!
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

    // Before the window is first realized, set it up to be a layer surface
    window.init_layer_shell();

    // Display above normal windows
    // window.set_layer(Layer::Overlay);
    window.set_layer(Layer::Top);

    // Push other windows out of the way
    // window.auto_exclusive_zone_enable();

    // The margins are the gaps around the window's edges
    // Margins and anchors can be set like this...
    // window.set_margin(Edge::Left, 40);
    // window.set_margin(Edge::Right, 40);
    // window.set_margin(Edge::Top, 20);

    // ... or like this
    // Anchors are if the window is pinned to each edge of the output
    let anchors = [
        (Edge::Left, true),
        (Edge::Right, false),
        (Edge::Top, false),
        (Edge::Bottom, true),
    ];

    for (anchor, state) in anchors {
        window.set_anchor(anchor, state);
    }

    draw_area::setup_draw(&window, size);

    window.present();
}
