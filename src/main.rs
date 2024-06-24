mod activate;
mod args;
mod config;
mod data;
mod ui;

use clap::Parser;
use gio::prelude::*;
use std::{thread, time::Duration};

fn main() {
    std::env::set_var("GSK_RENDERER", "cairo");

    let application = gtk::Application::new(None::<String>, Default::default());

    application.connect_activate(|app| {
        let args = args::Cli::parse();
        let group_map = config::get_config().unwrap();
        let cfgs = config::match_group_config(group_map, args.group);

        activate::activate(app, cfgs);
    });

    application.run();

    loop {
        thread::sleep(Duration::from_millis(1000));
    }
}
