mod data;
mod ui;

use gio::prelude::*;

// https://github.com/wmww/gtk-layer-shell/blob/master/examples/simple-example.c
fn activate(application: &gtk::Application) {
    ui::new_window(application, (data::RADIUS, data::LENGTH));
}

fn main() {
    std::env::set_var("GSK_RENDERER", "cairo");
    let application = gtk::Application::new(Some("sh.wmww.gtk-layer-example"), Default::default());

    application.connect_activate(|app| {
        activate(app);
    });

    application.run();
    // application.quit();
}
