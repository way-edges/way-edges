use std::{
    collections::HashMap,
    ops::DerefMut,
    sync::{Arc, Mutex, MutexGuard},
};

use backend::ipc::IPCCommand;
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
    pub groups: HashMap<String, Option<Group>>,

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
impl App {
    pub fn handle_ipc(&mut self, cmd: IPCCommand) {
        match cmd {
            IPCCommand::AddGroup(s) => self.add_group(&s),
            IPCCommand::RemoveGroup(s) => self.rm_group(&s),
            IPCCommand::TogglePin(gn, wn) => self.toggle_pin(&gn, &wn),
            IPCCommand::Exit => self.exit = true,
        };
    }

    fn add_group(&mut self, name: &str) {
        if self.groups.contains_key(name) {
            return;
        }
        let group = self.init_group(name);
        self.groups.insert(name.to_string(), group);
    }
    fn rm_group(&mut self, name: &str) {
        drop(self.groups.remove(name));
    }
    fn toggle_pin(&mut self, gn: &str, wn: &str) {
        let Some(Some(group)) = self.groups.get_mut(gn) else {
            return;
        };
        if let Some(mut w) = group.get_widget(wn) {
            w.toggle_pin()
        }
    }

    pub fn reload(&mut self) {
        // FIX: HOW CAN WE DO THIS???
        let ptr = self as *const App;
        for (k, widget_map) in self.groups.iter_mut() {
            drop(widget_map.take());
            *widget_map = unsafe { ptr.as_ref().unwrap() }.init_group(k.as_str());
        }
    }

    fn init_group(&self, name: &str) -> Option<Group> {
        let conf = config::get_config_by_group(Some(name));
        let res = conf.and_then(|vc| {
            let Some(vc) = vc else {
                return Err(format!("Not found config by group: {name}"));
            };
            log::debug!("group config:\n{vc:?}");
            Group::init_group(vc.widgets, self)
        });
        res.inspect_err(|e| {
            log::error!("{e}");
            util::notify_send("Way-edges app error", e, true);
        })
        .ok()
    }
}

pub struct Group {
    pub widgets: HashMap<String, Arc<Mutex<Widget>>>,
}
impl Group {
    fn init_group(widgets_config: Vec<config::Config>, app: &App) -> Result<Self, String> {
        todo!()
    }
    fn get_widget(&self, name: &str) -> Option<MutexGuard<Widget>> {
        self.widgets.get(name).map(|w| w.lock().unwrap())
    }
}

pub struct Widget {
    pub configured: bool,
    pub layer: LayerSurface,
}
impl Widget {
    fn toggle_pin(&mut self) {
        todo!()
    }
}

// TODO: we are not really access this in multithreaded situation, so we don't need
// Arc&Mutex, but since WlSurface::data needs Send&Sync, we might as well use it then.
// We can test for using Rc&RefCell, but it's not really a significant overhead when comparing to
// refresh rate(even 240hz still needs 4.9ms, but the overhead from lock is only nanoseconds)
// pub struct WidgetPtr(Rc<RefCell<Widget>>);
