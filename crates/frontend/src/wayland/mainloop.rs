use std::time::Duration;

use backend::{
    config_file_watch::start_configuration_file_watcher, ipc::start_ipc,
    runtime::init_backend_runtime_handle,
};
use calloop::EventLoop;
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    reexports::calloop_wayland_source::WaylandSource,
    registry::RegistryState,
    seat::SeatState,
    shell::wlr_layer::LayerShell,
    shm::{slot::SlotPool, Shm},
};
use wayland_client::{globals::registry_queue_init, Connection};

use crate::wayland::app::WidgetMap;

use super::app::App;

pub fn run_app(show_mouse_key: bool) {
    let conn = Connection::connect_to_env().unwrap();

    let (globals, event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    let mut event_loop: EventLoop<App> =
        EventLoop::try_new().expect("Failed to initialize the event loop!");
    let signal = event_loop.get_signal();
    let loop_handle = event_loop.handle();
    WaylandSource::new(conn.clone(), event_queue)
        .insert(loop_handle)
        .unwrap();

    let compositor_state =
        CompositorState::bind(&globals, &qh).expect("wl_compositor is not available");
    let layer_shell = LayerShell::bind(&globals, &qh).expect("layer shell is not available");
    let shm = Shm::bind(&globals, &qh).expect("wl_shm is not available");
    let pool = SlotPool::new(256 * 256 * 4, &shm).expect("Failed to create pool");
    let output_state = OutputState::new(&globals, &qh);
    let registry_state = RegistryState::new(&globals);
    let seat_state = SeatState::new(&globals, &qh);
    let fractional_manager = globals.bind(&qh, 0..=1, ()).into();
    let viewporter_manager = globals.bind(&qh, 0..=1, ()).into();

    let mut app = App {
        reload_guard: None,
        first_time_initialized: false,

        exit: false,
        show_mouse_key,
        queue_handle: qh,
        event_loop_handle: event_loop.handle(),
        signal,

        compositor_state,
        registry_state,
        seat_state,
        output_state,
        fractional_manager,
        viewporter_manager,
        shm,
        pool,
        pointer: None,
        shell: layer_shell,

        widget_map: WidgetMap::default(),
    };

    init_backend_runtime_handle();

    let (sender, r) = calloop::channel::channel();
    start_ipc(sender);
    event_loop
        .handle()
        .insert_source(r, |event, _, app| {
            let calloop::channel::Event::Msg(cmd) = event else {
                log::error!("IPC server shutdown, exiting...");
                app.exit = true;
                return;
            };
            app.handle_ipc(cmd);
        })
        .unwrap();

    let (sender, r) = calloop::channel::channel();
    start_configuration_file_watcher(sender);
    event_loop
        .handle()
        .insert_source(r, |event, _, app| {
            if let calloop::channel::Event::Closed = event {
                log::error!("IPC server shutdown, exiting...");
                app.exit = true;
                return;
            };
            app.reload();
        })
        .unwrap();

    event_loop.handle().insert_idle(|app| {
        app.reload();
    });

    while !app.exit {
        event_loop
            .dispatch(Some(Duration::from_millis(16)), &mut app)
            .unwrap();
    }
    log::info!("EXITED");
}
