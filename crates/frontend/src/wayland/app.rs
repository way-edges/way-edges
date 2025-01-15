use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use calloop::{LoopHandle, LoopSignal};
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    registry::RegistryState,
    seat::SeatState,
    shell::wlr_layer::{LayerShell, LayerSurface},
    shm::{slot::SlotPool, Shm},
};
use wayland_client::{protocol::wl_pointer, QueueHandle};

pub struct App {
    pub exit: bool,
    pub groups: HashMap<String, Group>,

    pub queue_handle: QueueHandle<App>,
    pub event_loop_handle: LoopHandle<'static, App>,
    pub signal: LoopSignal,

    pub compositor_state: CompositorState,
    pub registry_state: RegistryState,
    pub output_state: OutputState,
    pub seat_state: SeatState,
    pub pointer: Option<wl_pointer::WlPointer>,

    pub shell: LayerShell,
    pub shm: Shm,
    pub pool: SlotPool,
}

pub struct Group {
    pub widgets: HashMap<String, Arc<Mutex<Widget>>>,
}

pub struct Widget {
    pub configured: bool,
    pub layer: LayerSurface,
}

// TODO: we are not really access this in multithreaded situation, so we don't need
// Arc&Mutex, but since WlSurface::data needs Send&Sync, we might as well use it then.
// We can test for using Rc&RefCell, but it's not really a significant overhead when comparing to
// refresh rate(even 240hz still needs 4.9ms, but the overhead from lock is only nanoseconds)
// pub struct WidgetPtr(Rc<RefCell<Widget>>);
