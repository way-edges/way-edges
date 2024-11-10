use gio::prelude::*;
use gtk::gdk::Monitor;
use gtk::prelude::{DisplayExt, MonitorExt};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicPtr};

use crate::notify_send;

use super::GroupMapCtxRc;

#[derive(Debug, Clone, Deserialize)]
pub enum MonitorSpecifier {
    ID(usize),
    Name(String),
}

pub struct MonitorCtx {
    pub monitors: Vec<Monitor>,
    pub name_index_map: HashMap<String, usize>,
}
impl MonitorCtx {
    fn new() -> Self {
        Self {
            monitors: Vec::new(),
            name_index_map: HashMap::new(),
        }
    }

    pub fn get_monitor(&self, specifier: &MonitorSpecifier) -> Option<&Monitor> {
        match specifier {
            MonitorSpecifier::ID(index) => self.monitors.get(*index),
            MonitorSpecifier::Name(name) => self.monitors.get(*self.name_index_map.get(name)?),
        }
    }

    pub fn get_monitor_size(&self, specifier: &MonitorSpecifier) -> Option<(i32, i32)> {
        let monitor = self.get_monitor(specifier)?;
        let geom = monitor.geometry();
        Some((geom.width(), geom.height()))
    }

    fn reload_monitors(&mut self) -> Result<(), String> {
        let default_display =
            gtk::gdk::Display::default().ok_or("display for monitor not found")?;

        self.monitors = default_display
            .monitors()
            .iter::<Monitor>()
            .map(|m| m.map_err(|e| format!("Get monitor error: {e}")))
            .collect::<Result<Vec<Monitor>, String>>()?;

        self.name_index_map = self
            .monitors
            .iter()
            .enumerate()
            .map(|(index, monitor)| {
                let a = monitor
                    .connector()
                    .ok_or(format!("Fail to get monitor connector name: {monitor:?}"))?;
                Ok((a.to_string(), index))
            })
            .collect::<Result<HashMap<String, usize>, String>>()?;

        Ok(())
    }
}

pub static MONITORS: AtomicPtr<MonitorCtx> = AtomicPtr::new(std::ptr::null_mut());

pub fn get_monitor_context() -> &'static mut MonitorCtx {
    return unsafe {
        MONITORS
            .load(std::sync::atomic::Ordering::Acquire)
            .as_mut()
            .unwrap()
    };
}

pub fn init_monitor(group_map: GroupMapCtxRc) -> Result<(), String> {
    static IS_MONITOR_WATCHER_INITED: AtomicBool = AtomicBool::new(false);
    if IS_MONITOR_WATCHER_INITED.load(std::sync::atomic::Ordering::Acquire) {
        return Err("Monitor watcher already initialized".to_string());
    }

    let mut ctx = MonitorCtx::new();
    ctx.reload_monitors()?;

    MONITORS.store(
        Box::into_raw(Box::new(ctx)),
        std::sync::atomic::Ordering::Release,
    );

    let monitor_changed_signal_receiver = monitor_watch::start_monitor_watcher();

    gtk::glib::spawn_future_local(async move {
        loop {
            match monitor_changed_signal_receiver.recv().await {
                Ok(_) => {
                    group_map.borrow_mut().reload();
                }
                Err(e) => {
                    let msg = format!("Monitor Watcher receiver break with error: {e:?}");
                    log::error!("{msg}");
                    notify_send("Monitor Watcher", msg.as_str(), true);
                    break;
                }
            }
        }
    });

    Ok(())
}

mod monitor_watch {

    use async_channel::{Receiver, Sender};
    use smithay_client_toolkit::reexports::calloop::EventLoop;
    use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
    use smithay_client_toolkit::reexports::client::{
        globals::registry_queue_init, protocol::wl_output, Connection, QueueHandle,
    };
    use smithay_client_toolkit::{
        delegate_output, delegate_registry,
        output::{OutputHandler, OutputState},
        registry::{ProvidesRegistryState, RegistryState},
        registry_handlers,
    };

    use crate::activate::monitor::get_monitor_context;
    use crate::{get_main_runtime_handle, notify_send};

    pub fn start_monitor_watcher() -> Receiver<()> {
        let (s, r) = async_channel::bounded(1);
        std::thread::spawn(move || {
            MonitorWatcher::spawn(s);
        });
        r
    }

    struct MonitorWatcher {
        signal_sender: Sender<()>,
        output_state: OutputState,
        registry_state: RegistryState,
        last_task: Option<tokio::task::AbortHandle>,
    }
    impl MonitorWatcher {
        fn spawn(monitor_changed_signal_sender: Sender<()>) {
            log::info!("Start monitor watcher");

            // All Wayland apps start by connecting the compositor (server).
            let conn = Connection::connect_to_env().unwrap();

            // Enumerate the list of globals to get the protocols the server implements.
            let (globals, event_queue) = registry_queue_init(&conn).unwrap();
            let qh = event_queue.handle();
            let mut event_loop: EventLoop<Self> =
                EventLoop::try_new().expect("Failed to initialize the event loop!");
            let loop_handle = event_loop.handle();
            WaylandSource::new(conn.clone(), event_queue)
                .insert(loop_handle)
                .unwrap();

            // Initialize the registry handling
            let registry_state = RegistryState::new(&globals);

            // Initialize the delegate we will use for outputs.
            let output_state = OutputState::new(&globals, &qh);

            let mut ll = Self {
                signal_sender: monitor_changed_signal_sender,
                output_state,
                registry_state,
                last_task: None,
            };

            loop {
                if let Err(e) = event_loop.dispatch(None, &mut ll) {
                    let msg = format!("Monitor watcher event loop Error: {e}");
                    log::error!("{msg}");
                    notify_send("Way-Edges Monitor Watcher", &msg, true);
                    break;
                };
            }
        }

        fn on_change(&mut self) {
            log::debug!("received monitor change event");

            if let Some(last_task) = self.last_task.take() {
                last_task.abort();
            }

            let signal_sender = self.signal_sender.clone();
            self.last_task = Some(
                get_main_runtime_handle()
                    .spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(5000)).await;

                        let _ = get_monitor_context().reload_monitors();

                        if let Err(e) = signal_sender.force_send(()) {
                            let msg = format!("monitor changed signal failed to send: {e}");
                            log::error!("{msg}");
                            notify_send("Way-Edges Monitor Watcher", &msg, true);
                        }
                    })
                    .abort_handle(),
            );
        }
    }

    impl OutputHandler for MonitorWatcher {
        fn output_state(&mut self) -> &mut OutputState {
            &mut self.output_state
        }

        fn new_output(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _output: wl_output::WlOutput,
        ) {
            log::debug!("New wayland output: {_output:?}");
            self.on_change();
        }

        fn update_output(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _output: wl_output::WlOutput,
        ) {
            log::debug!("update wayland output: {_output:?}");
            self.on_change();
        }

        fn output_destroyed(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _output: wl_output::WlOutput,
        ) {
            log::debug!("remove wayland output: {_output:?}");
            self.on_change();
        }
    }

    impl ProvidesRegistryState for MonitorWatcher {
        fn registry(&mut self) -> &mut RegistryState {
            &mut self.registry_state
        }

        registry_handlers! {
            OutputState,
        }
    }

    delegate_output!(MonitorWatcher);
    delegate_registry!(MonitorWatcher);
}
