use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    io,
    path::Path,
    rc::Rc,
    thread,
};

use gio::{
    glib::{self, clone::Downgrade, WeakRef},
    prelude::{ApplicationExt, ApplicationExtManual},
    ApplicationFlags, ApplicationHoldGuard,
};
use gtk::Application;
use log::debug;
use tokio::net::UnixStream;

use crate::{
    activate::{self, GroupCtx},
    config, init_file_monitor,
    ipc_command::{
        CommandBody, IPCCommand, IPC_COMMAND_ADD, IPC_COMMAND_QUIT, IPC_COMMAND_REMOVE,
        IPC_COMMAND_TOGGLE_PIN,
    },
    notify_send, start_watch_file, stop_watch_file,
};

// pub const TEMP_DIR: &str = "/tmp/way-edges";
// pub const LOCK_FILE: &str = "way-edges.lock";
pub const SOCK_FILE: &str = "/tmp/way-edges/way-edges.sock";

// fn init_app(app: &Application, reload_signal_receiver: &Receiver<i32>) {
fn init_group(app: &Application, name: &str) -> Result<Box<dyn GroupCtx>, String> {
    stop_watch_file();
    let conf = config::get_config(Some(name));
    start_watch_file();
    let res = conf.and_then(|vc| {
        debug!("Parsed Config: {vc:?}");
        {
            #[cfg(feature = "hyprland")]
            {
                use activate::hyprland::Hyprland;
                Hyprland::init_window(app, cfgs)
            }
            #[cfg(not(feature = "hyprland"))]
            {
                use activate::default::Default;
                Default::init_window(app, vc)
            }
        }
    });
    match res {
        Ok(v) => Ok(Box::new(v)),
        Err(e) => {
            log::error!("{e}");
            crate::notify_send("Way-edges app error", &e, true);
            Err(e)
        }
    }
}

fn on_active(app: &Application) {
    if let Err(e) = activate::init_monitor() {
        let msg = format!("Failed to init monitor: {e}");
        notify_send("Way-edges monitor", &msg, true);
        app.quit();
    };
}

struct GroupMapCtx {
    map: GroupMap,
    app: Option<WeakRef<Application>>,
    hold: Option<ApplicationHoldGuard>,
}
impl GroupMapCtx {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            app: None,
            hold: None,
        }
    }
    fn inited(&mut self, app: &Application) {
        self.hold = Some(app.hold());
        self.app = Some(Downgrade::downgrade(app));
        if !self.map.is_empty() {
            self.reload();
        }
    }
    fn add_group(&mut self, name: &str) {
        if !self.map.contains_key(name) {
            if let Some(app) = &self.app {
                let s = init_group(&app.upgrade().unwrap(), name).ok();
                self.map.insert(name.to_string(), s);
            } else {
                self.map.insert(name.to_string(), None);
            }
        }
    }
    fn rm_group(&mut self, name: &str) {
        if let Some(Some(mut v)) = self.map.remove(name) {
            v.close()
        }
    }
    fn reload(&mut self) {
        if let Some(app) = &self.app {
            let app = app.upgrade().unwrap();
            self.map.iter_mut().for_each(|(k, v)| {
                if let Some(mut v) = v.take() {
                    v.close()
                }
                let a = init_group(&app, k.as_str());
                *v = a.ok();
            });
        }
    }
    fn dispose(&mut self) {
        self.map.iter_mut().for_each(|(_, v)| {
            if let Some(v) = v.as_mut() {
                v.close()
            }
        });
        if let Some(app) = &self.app {
            if let Some(app) = app.upgrade() {
                app.quit()
            }
        }
        drop(self.hold.take());
    }
    fn toggle_pin(&mut self, gn: &str, wn: &str) {
        if let Some(Some(v)) = self.map.get_mut(gn) {
            if let Some(v) = v.widget_map().get_mut(wn) {
                v.widget_expose.toggle_pin()
            }
        }
    }
}

type GroupMap = HashMap<String, Option<Box<dyn GroupCtx>>>;
type GroupMapCtxRc = Rc<RefCell<GroupMapCtx>>;
fn new_app() -> (GroupMapCtxRc, Application) {
    // that flag is for command line arguments
    let app = gtk::Application::new(Some("com.ogios.way-edges"), ApplicationFlags::HANDLES_OPEN);
    // gtk::Application::new(None::<String>, ApplicationFlags::HANDLES_OPEN);

    let group_map = Rc::new(RefCell::new(GroupMapCtx::new()));

    let is_already_active = Rc::new(Cell::new(false));

    // when args passed, `open` will be signaled instead of `activate`
    app.connect_open(glib::clone!(
        #[weak]
        group_map,
        #[strong]
        is_already_active,
        move |app, _, _| {
            if is_already_active.get() {
                notify_send(
                    "Way-edges",
                    "A way-edges daemon already running, something trys to run one more",
                    true,
                );
            } else {
                is_already_active.set(true);
                debug!("connect open");
                on_active(app);
                group_map.borrow_mut().inited(app);
            }
        }
    ));
    app.connect_activate(glib::clone!(
        #[weak]
        group_map,
        move |app| {
            if is_already_active.get() {
                notify_send(
                    "Way-edges",
                    "A way-edges daemon already running, something trys to run one more",
                    true,
                );
            } else {
                is_already_active.set(true);
                debug!("connect activate");
                on_active(app);
                group_map.borrow_mut().inited(app);
            }
        }
    ));
    (group_map, app)
}

pub async fn daemon() {
    // set renderer explicitly to cairo instead of ngl
    std::env::set_var("GSK_RENDERER", "cairo");

    // NOTE: `notify` takes 2 thread, may be i can make it to main tokio thread?
    // idk how to do it.
    let file_change_signal_receiver = init_file_monitor();

    let (ipc_command_sender, ipc_command_receiver) = async_channel::unbounded::<IPCCommand>();

    // this is where glib mainloop will be
    // NOTE: glib here 2 threads, also glib it self needs lots of other thread
    // normally till here it will be 10-12 threads already (1 + 2 + 2 + <glib/gio additional threads>)
    let glib_mainloop = thread::spawn(move || {
        let (group_ctx, app) = new_app();
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
        glib::spawn_future_local(async move {
            while let Ok(command) = ipc_command_receiver.recv().await {
                log::debug!("recv ipc command: {command:?}");
                match command {
                    IPCCommand::AddGroup(s) => {
                        group_ctx.borrow_mut().add_group(&s);
                    }
                    IPCCommand::RemoveGroup(s) => {
                        group_ctx.borrow_mut().rm_group(&s);
                    }
                    IPCCommand::Exit => {
                        log::info!("dispose");
                        group_ctx.borrow_mut().dispose();
                        ipc_command_receiver.close();
                    }
                    IPCCommand::TogglePin(gn, wn) => group_ctx.borrow_mut().toggle_pin(&gn, &wn),
                }
            }
        });
        if app.run_with_args::<String>(&[]).value() == 1 {
            notify_send(
                "Way-edges",
                "Application exit unexpectedly, it's likely a gtk4 issue",
                true,
            );
        };
        log::info!("Application exit");
    });

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

    let listener = {
        let path = Path::new(SOCK_FILE);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let _ = std::fs::remove_file(SOCK_FILE);
        tokio::net::UnixListener::bind(SOCK_FILE).unwrap()
    };

    let ipc_task = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    deal_stream(stream, ipc_command_sender.clone());
                }
                Err(e) => {
                    let msg = format!("Fail to connect socket: {e}");
                    notify_send("Way-edges", msg.as_str(), true);
                    log::error!("msg");
                    break;
                }
            }
        }
    });
    tokio::select! {
        _ = ipc_task => {},
        _ = glib_mainloop_exit_receiver => {}
    };
    log::info!("Sock listener exit");
    std::fs::remove_file(SOCK_FILE).unwrap();
}

fn deal_stream(stream: UnixStream, sender: async_channel::Sender<IPCCommand>) {
    tokio::spawn(async move {
        let raw = stream_read_all(&stream).await?;
        log::debug!("recv ipc msg: {raw}");
        let command_body =
            serde_jsonrc::from_str::<CommandBody>(&raw).map_err(|e| e.to_string())?;
        let ipc = match command_body.command.as_str() {
            IPC_COMMAND_ADD => {
                IPCCommand::AddGroup(command_body.args.first().ok_or("No group name")?.clone())
            }
            IPC_COMMAND_REMOVE => {
                IPCCommand::RemoveGroup(command_body.args.first().ok_or("No group name")?.clone())
            }
            IPC_COMMAND_TOGGLE_PIN => IPCCommand::TogglePin(
                command_body.args.first().ok_or("No group name")?.clone(),
                command_body.args.get(1).ok_or("No widget name")?.clone(),
            ),
            IPC_COMMAND_QUIT => IPCCommand::Exit,
            _ => return Err("unknown command".to_string()),
        };
        sender
            .send(ipc)
            .await
            .map_err(|_| "ipc channel closed".to_string())?;
        Ok(())
    });
}

async fn stream_read_all(stream: &UnixStream) -> Result<String, String> {
    let mut buf_array = vec![];
    let a = loop {
        // Wait for the socket to be readable
        if stream.readable().await.is_err() {
            return Err("stream not readable".to_string());
        }

        // Creating the buffer **after** the `await` prevents it from
        // being stored in the async task.
        let mut buf = [0; 4096];

        // Try to read data, this may still fail with `WouldBlock`
        // if the readiness event is a false positive.
        match stream.try_read(&mut buf) {
            Ok(0) => break String::from_utf8_lossy(&buf_array),
            Ok(n) => {
                buf_array.extend_from_slice(&buf[..n]);
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                return Err(format!("Can not read command: {e}"));
            }
        }
    };

    Ok(a.to_string())
}
