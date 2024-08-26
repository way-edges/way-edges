use std::{
    collections::HashMap,
    process,
    str::FromStr,
    sync::{Mutex, MutexGuard, OnceLock},
    thread,
};

use hyprland::{
    event_listener::{self, WindowEventData},
    shared::{HyprData, HyprDataActive, WorkspaceType},
};

use crate::notify_send;

fn notify_hyprland_log(msg: &str, is_critical: bool) {
    notify_send("Way-Edges Hyprland error", msg, is_critical);
    log::error!("{msg}");

    if is_critical {
        process::exit(-1)
    }
}

pub enum HyprEvent {
    Workspace(i32),
    ActiveWindow(WindowEventData),
}

pub type HyprCallbackId = u32;
pub type HyprCallback = Box<dyn 'static + FnMut(&HyprGlobalData)>;

#[derive(Debug, Clone, Copy)]
pub struct HyprGlobalData {
    pub max_workspace: i32,
    pub current_workspace: i32,
    pub last_workspace: i32,
}
impl Default for HyprGlobalData {
    fn default() -> Self {
        Self {
            max_workspace: Default::default(),
            current_workspace: Default::default(),
            last_workspace: Default::default(),
        }
    }
}
impl HyprGlobalData {
    fn new() -> Self {
        let mut s = Self {
            max_workspace: 0,
            current_workspace: 0,
            last_workspace: 0,
        };
        s.reload_max_worksapce();

        s.current_workspace = match hyprland::data::Workspace::get_active() {
            Ok(w) => w.id,
            Err(e) => {
                notify_hyprland_log(
                    format!("Failed to find active workspace: {e}").as_str(),
                    true,
                );
                unreachable!();
            }
        };

        s
    }
    fn move_current(&mut self, id: i32) {
        self.last_workspace = self.current_workspace;
        self.current_workspace = id;
    }
    fn reload_max_worksapce(&mut self) {
        match hyprland::data::Workspaces::get() {
            Ok(ws) => {
                let max_workspace =
                    ws.into_iter()
                        .rev()
                        .find_map(|w| if w.id > 0 { Some(w.id) } else { None });
                if let Some(id) = max_workspace {
                    self.max_workspace = id;
                } else {
                    notify_hyprland_log("Failed to find available workspace", true);
                }
            }
            Err(e) => {
                notify_hyprland_log(format!("Failed to reload workspaces: {e}").as_str(), true);
            }
        }
    }
}

struct HyprListenerCtx {
    id_cache: u32,
    cb: HashMap<HyprCallbackId, HyprCallback>,
    data: HyprGlobalData,
}

impl HyprListenerCtx {
    fn new() -> Self {
        Self {
            cb: HashMap::new(),
            id_cache: 0,

            data: HyprGlobalData::new(),
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
        match e {
            HyprEvent::Workspace(s) => self.data.move_current(s),
            HyprEvent::ActiveWindow(_) => {}
        };
        self.cb.values_mut().for_each(|f| {
            f(&self.data);
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
            match i32::from_str(&id) {
                Ok(int) => {
                    get_hypr_listener().call(HyprEvent::Workspace(int));
                }
                Err(e) => {
                    notify_hyprland_log(format!("Fail to parse workspace id: {e}").as_str(), false)
                }
            }
        }
    });
    listener.add_workspace_added_handler(|id| {
        if let WorkspaceType::Regular(_) = id {
            get_hypr_listener().data.reload_max_worksapce()
        }
    });
    listener.add_workspace_destroy_handler(|_| {
        get_hypr_listener().data.reload_max_worksapce();
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

pub fn register_hypr_event_callback(
    cb: impl FnMut(&HyprGlobalData) + 'static,
) -> (HyprCallbackId, HyprGlobalData) {
    let mut hypr = get_hypr_listener();
    (hypr.add_cb(Box::new(cb)), hypr.data.clone())
}

pub fn unregister_hypr_event_callback(id: HyprCallbackId) {
    get_hypr_listener().remove_cb(id)
}
