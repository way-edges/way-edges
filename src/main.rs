mod activate;
mod args;
mod config;
mod file_watch;
mod ui;

use std::{process, thread};

use activate::WindowDestroyer;
use activate::WindowInitializer;
use async_channel::Receiver;
use file_watch::*;
use gio::glib::idle_add_local_once;
use gio::{prelude::*, ApplicationFlags};
use gtk::glib;
use gtk::Application;
use log::debug;
use notify_rust::Notification;

fn main() {
    env_logger::builder().format_suffix("\n\n").init();
    // for cmd line help msg.
    // or else it will show help from `gtk` other than `clap`
    args::get_args();

    // set renderer explicitly to cairo instead of ngl
    std::env::set_var("GSK_RENDERER", "cairo");

    let file_change_signal_receiver = init_file_monitor();

    let (reload_signal_sender, reload_signal_receiver) = async_channel::bounded(1);
    let (continue_sender, continue_receiver) = async_channel::bounded(1);

    thread::spawn(move || loop {
        if let Err(e) = file_change_signal_receiver.recv_blocking() {
            let msg = format!("File change signal Error: {e}");
            log::error!("{msg}");
            notify_send("Way-edges file change signal", &msg, true);
            process::exit(1);
        } else {
            debug!("Receive file change signal");
            reload_signal_sender.try_send(()).ok();
            if let Err(e) = continue_sender.send_blocking(()) {
                let msg = format!("Reload conitnue siganl Error: {e}");
                log::error!("{msg}");
                notify_send("Way-edges reload conitnue signal", &msg, true);
                process::exit(1);
            };
        }
    });

    #[allow(clippy::never_loop)]
    loop {
        stop_watch_file();
        let res = config::init_config();
        start_watch_file();

        if let Err(e) = res {
            log::error!("{e}");
            notify_send("Way-edges init config", &e, true);
        } else {
            // that flag is for command line arguments
            let application =
            // gtk::Application::new(Some("com.ogios.way-edges"), ApplicationFlags::HANDLES_OPEN);
            gtk::Application::new(None::<String>, ApplicationFlags::HANDLES_OPEN);

            // when args passed, `open` will be signaled instead of `activate`
            application.connect_open(
                glib::clone!(@strong reload_signal_receiver as r  =>  move |app, _, _| {
                    debug!("connect open");
                    init_app(app, &r);
                }),
            );
            application.connect_activate(
                glib::clone!(@strong reload_signal_receiver as r  =>  move |app| {
                    debug!("connect activate");
                    init_app(app, &r);
                }),
            );
            if application.run_with_args::<String>(&[]).value() == 1 {
                notify_send(
                    "Way-edges",
                    "Application exit unexpectedly, it's likely a gtk4 issue",
                    true,
                );
                break;
            };
        }

        debug!("WAIT FOR CONITNUE...");
        if continue_receiver.recv_blocking().is_err() {
            notify_send(
                "Way-edges reload conitnue signal",
                "Channel exit unexpectedly",
                true,
            );
            break;
        }
        log::debug!("Reload!!!");
        notify_send("Way-edges", "App Reload", false);
    }
}

fn init_app(app: &Application, error_signal_receiver: &Receiver<()>) {
    let args = args::get_args();
    debug!("Parsed Args: {:?}", args);
    let cfgs = match config::take_config() {
        Ok(v) => v,
        Err(e) => {
            notify_send(
                "Way-edges config",
                &format!("Failed to load config: {e}"),
                true,
            );
            return;
        }
    };
    // let cfgs = config::match_group_config(group_map, &args.group);
    debug!("Parsed Config: {cfgs:?}");
    match activate::init_monitor() {
        Ok(_) => {}
        Err(e) => {
            notify_send(
                "Way-edges monitor",
                &format!("Failed to init monitor: {e}"),
                true,
            );
            return;
        }
    };
    let res = {
        #[cfg(feature = "hyprland")]
        {
            use activate::hyprland::Hyprland;
            Hyprland::init_window(app, cfgs)
        }
        #[cfg(not(feature = "hyprland"))]
        {
            use activate::default::Default;
            Default::init_window(app, cfgs)
        }
    };
    let window_destroyer = match res {
        Ok(v) => v,
        Err(e) => {
            log::error!("{e}");
            crate::notify_send("Way-edges app error", &e, true);
            return;
        }
    };

    glib::spawn_future_local(
        glib::clone!(@weak app, @strong error_signal_receiver as r => async move {
            if r.recv().await.is_ok() {
                log::info!("Received reload signal, quiting..");
                window_destroyer.close_window();
                idle_add_local_once(glib::clone!(@weak app => move || {
                    debug!("Quit app");
                    app.quit();
                }));
            }
        }),
    );
}
pub fn notify_send(summary: &str, body: &str, is_critical: bool) {
    let mut n = Notification::new();
    n.summary(summary);
    n.body(body);
    if is_critical {
        n.urgency(notify_rust::Urgency::Critical);
    }
    if let Err(e) = n.show() {
        log::error!("Failed to send notification: \"{summary}\" - \"{body}\"\nError: {e}");
    }
}
