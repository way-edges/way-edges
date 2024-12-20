use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    thread,
};

use gio::{
    glib::{self},
    prelude::{ApplicationExt, ApplicationExtManual},
    ApplicationFlags,
};
use gtk::Application;
use log::debug;

use crate::{
    activate::{GroupMapCtx, GroupMapCtxRc},
    get_main_runtime_handle,
};
use util::notify_send;

fn monitor_change_cb(group_map: &GroupMapCtxRc, app: &gtk::Application) -> bool {
    let debouncer_context: Cell<Option<Rc<Cell<bool>>>> = Cell::new(None);
    let cb = gtk::glib::clone!(
        #[weak]
        group_map,
        move |_: &'_ gio::ListModel, _, _, _| {
            log::info!("Monitor changed");
            use gtk::glib;

            if let Some(removed) = debouncer_context.take() {
                removed.set(true);
            }

            let removed = Rc::new(Cell::new(false));

            gtk::glib::timeout_add_seconds_local_once(
                5,
                glib::clone!(
                    #[weak]
                    group_map,
                    #[weak]
                    removed,
                    move || {
                        if removed.get() {
                            return;
                        }

                        if let Err(e) = backend::monitor::get_monitor_context().reload_monitors() {
                            let msg = format!("Fail to reload monitors: {e}");
                            log::error!("{msg}");
                            notify_send("Monitor Watcher", &msg, true);
                        }
                        group_map.borrow_mut().reload();
                    }
                ),
            );
            debouncer_context.set(Some(removed.clone()))
        }
    );

    if let Err(e) = backend::monitor::init_monitor(cb) {
        let msg = format!("Failed to init monitor: {e}");
        notify_send("Way-edges monitor", &msg, true);
        app.quit();
        true
    } else {
        false
    }
}

fn new_app() -> (GroupMapCtxRc, Application) {
    // that flag is for command line arguments
    let app = gtk::Application::new(Some("com.ogios.way-edges"), ApplicationFlags::HANDLES_OPEN);

    let group_map = Rc::new(RefCell::new(GroupMapCtx::new()));

    let is_already_active = Rc::new(Cell::new(false));

    let on_app_start = glib::clone!(
        #[weak]
        group_map,
        #[to_owned]
        is_already_active,
        move |app: &gtk::Application| {
            if is_already_active.get() {
                notify_send(
                    "Way-edges",
                    "A way-edges daemon already running, something trys to run one more",
                    true,
                );
            } else {
                is_already_active.set(true);

                debug!("connect open or activate");

                // group map
                group_map.borrow_mut().init_with_app(app);

                // monitor
                if monitor_change_cb(&group_map, app) {
                    return;
                };

                let mut group_map_mut = group_map.borrow_mut();
                if !group_map_mut.map.is_empty() {
                    group_map_mut.reload();
                }
            }
        }
    );

    // when args passed, `open` will be signaled instead of `activate`
    app.connect_open(glib::clone!(
        #[strong]
        on_app_start,
        move |app, _, _| {
            on_app_start(app);
        }
    ));
    app.connect_activate(on_app_start);

    (group_map, app)
}

pub async fn daemon() {
    // set renderer explicitly to cairo instead of ngl
    std::env::set_var("GSK_RENDERER", "cairo");

    let file_change_signal_receiver = backend::config_file_watch::init_config_file_monitor();

    let (ipc_join_handle_sender, ipc_join_handle_receiver) = tokio::sync::oneshot::channel();

    let glib_mainloop = thread::spawn(move || {
        let (group_ctx, app) = new_app();

        // config file change
        glib::spawn_future_local(glib::clone!(
            #[weak]
            group_ctx,
            async move {
                while (file_change_signal_receiver.recv().await).is_ok() {
                    group_ctx.borrow_mut().reload();
                    log::debug!("Reload!!!");
                    notify_send("Way-edges", "App Reload", false);
                }
                log::error!("File Watcher exit");
                notify_send("Way-edges file watcher", "watcher exited", true);
            }
        ));

        // ipc
        let (ipc_join_handle, processer) =
            get_main_runtime_handle().block_on(ipc::listen_ipc(move |command| {
                log::debug!("recv ipc command: {command:?}");
                match command {
                    ipc::IPCCommand::AddGroup(s) => {
                        group_ctx.borrow_mut().add_group(&s);
                    }
                    ipc::IPCCommand::RemoveGroup(s) => {
                        group_ctx.borrow_mut().rm_group(&s);
                    }
                    ipc::IPCCommand::Exit => {
                        log::info!("dispose");
                        group_ctx.borrow_mut().dispose();
                    }
                    ipc::IPCCommand::TogglePin(gn, wn) => {
                        group_ctx.borrow_mut().toggle_pin(&gn, &wn)
                    }
                };
            }));
        glib::spawn_future_local(processer);
        ipc_join_handle_sender.send(ipc_join_handle).unwrap();

        if app.run_with_args::<String>(&[]).value() == 1 {
            notify_send(
                "Way-edges",
                "Application exit unexpectedly, it's likely a gtk4 issue",
                true,
            );
        };

        log::info!("Application exit");
    });

    // capture when glib exit
    let (glib_mainloop_exit_sender, glib_mainloop_exit_receiver) = tokio::sync::oneshot::channel();
    thread::spawn(move || {
        match glib_mainloop.join() {
            Ok(_) => {
                notify_send("Way-edges", "Glib mainloop Exit", true);
            }
            Err(e) => {
                let msg = format!("Glib mainloop Exit with error: {e:?}");
                notify_send("Way-edges", msg.as_str(), true);
                log::error!("{msg}");
            }
        };
        glib_mainloop_exit_sender.send(()).unwrap();
    });

    // ipc handle
    let ipc_join_handle = ipc_join_handle_receiver.await.unwrap();

    tokio::select! {
        _ = ipc_join_handle => {},
        _ = glib_mainloop_exit_receiver => {}
    };
    log::info!("Sock listener exit");
}
