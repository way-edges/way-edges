use std::{
    collections::HashMap,
    process,
    sync::{Mutex, MutexGuard, OnceLock},
    thread,
};

use hyprland::{
    event_listener::{self, WindowEventData},
    shared::WorkspaceType,
};

use crate::notify_send;

pub enum HyprEvent {
    Workspace(String),
    ActiveWindow(WindowEventData),
}

pub type HyprCallbackId = u32;
pub type HyprCallback = Box<dyn Send + 'static + FnMut(&HyprEvent)>;

struct HyprListenerCtx {
    id_cache: u32,
    cb: HashMap<HyprCallbackId, HyprCallback>,
}

impl HyprListenerCtx {
    fn new() -> Self {
        Self {
            cb: HashMap::new(),
            id_cache: 0,
        }
    }
    fn add_cb(&mut self, cb: HyprCallback) -> HyprCallbackId {
        let id = self.id_cache;
        self.cb.insert(id, cb);
        self.id_cache += 1;
        id
    }
    fn remove_cb(&mut self, id: HyprCallbackId) {
        self.cb.remove(&id);
    }
    fn call(&mut self, e: HyprEvent) {
        self.cb.values_mut().for_each(|f| {
            f(&e);
        })
    }
}
unsafe impl Send for HyprListenerCtx {}
unsafe impl Sync for HyprListenerCtx {}

static GLOBAL_HYPR_LISTENER_CTX: OnceLock<Mutex<HyprListenerCtx>> = OnceLock::new();

fn get_hypr_listener() -> MutexGuard<'static, HyprListenerCtx> {
    GLOBAL_HYPR_LISTENER_CTX
        .get_or_init(|| Mutex::new(HyprListenerCtx::new()))
        .lock()
        .unwrap()
}

pub fn init_hyprland_listener() {
    if GLOBAL_HYPR_LISTENER_CTX.get().is_some() {
        return;
    }

    let mut listener = event_listener::EventListener::new();
    listener.add_workspace_change_handler(|id| {
        if let WorkspaceType::Regular(id) = id {
            get_hypr_listener().call(HyprEvent::Workspace(id));
        }
    });
    listener.add_active_window_change_handler(|id| {
        if let Some(e) = id {
            get_hypr_listener().call(HyprEvent::ActiveWindow(e));
        }
    });

    thread::spawn(move || {
        if let Err(e) = listener.start_listener() {
            notify_send("Way-Edges Hyprland error", e.to_string().as_str(), true);
            process::exit(-1)
        }
    });
}

pub fn register_hypr_event_callback(cb: impl FnMut(&HyprEvent) + Send + 'static) -> HyprCallbackId {
    get_hypr_listener().add_cb(Box::new(cb))
}

pub fn unregister_hypr_event_callback(id: HyprCallbackId) {
    get_hypr_listener().remove_cb(id)
}
