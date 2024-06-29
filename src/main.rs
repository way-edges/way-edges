mod activate;
mod args;
mod config;
mod ui;

use activate::WindowInitializer;
use gio::{prelude::*, ApplicationFlags};
use gtk::Application;

fn main() {
    // for cmd line help msg.
    // or else it will show help from `gtk` other than `clap`
    args::get_args();

    // set renderer explicitly to cairo instead of ngl
    std::env::set_var("GSK_RENDERER", "cairo");

    // that flag is for command line arguments
    let application =
        gtk::Application::new(Some("com.ogios.way-edges"), ApplicationFlags::HANDLES_OPEN);

    // when args passed, `open` will be signaled instead of `activate`
    application.connect_open(|app, _, _| {
        init_app(app);
    });
    application.connect_activate(|app| {
        init_app(app);
    });

    application.run_with_args::<String>(&[]);
}

fn init_app(app: &Application) {
    let args = args::get_args();
    println!("{:#?}", args);
    let cfgs = config::get_config(&args.group).unwrap();
    // let cfgs = config::match_group_config(group_map, &args.group);
    cfgs.iter().for_each(|c| {
        println!("{c:#?}");
    });

    #[cfg(feature = "hyprland")]
    {
        use activate::hyprland::Hyprland;
        Hyprland::init_window(app, cfgs);
    }
    #[cfg(not(feature = "hyprland"))]
    {
        use activate::default::Default;
        Default::init_window(app, cfgs);
    }
}
