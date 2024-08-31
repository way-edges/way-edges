use std::{collections::HashMap, num::ParseIntError, process, str::FromStr, thread};

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

#[derive(Debug, Clone, Copy, Default)]
pub struct HyprGlobalData {
    pub max_workspace: i32,
    pub current_workspace: i32,
    pub last_workspace: i32,
}
impl HyprGlobalData {
    fn new() -> Self {
        let mut s = Self {
            max_workspace: 0,
            current_workspace: 0,
            last_workspace: 0,
        };
        s.reload_max_worksapce();
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

                log::debug!("reload hyprland max workspace: {max_workspace:?}");

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

        match hyprland::data::Workspace::get_active() {
            Ok(w) => self.current_workspace = w.id,
            Err(e) => {
                notify_hyprland_log(
                    format!("Failed to find active workspace: {e}").as_str(),
                    true,
                );
            }
        };
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
    fn call_event(&mut self, e: HyprEvent) {
        match e {
            HyprEvent::Workspace(s) => self.data.move_current(s),
            HyprEvent::ActiveWindow(_) => {}
        };
        self.call();
    }
    fn call(&mut self) {
        self.cb.values_mut().for_each(|f| {
            f(&self.data);
        })
    }
    fn reload(&mut self) {
        self.data.reload_max_worksapce();
        self.call();
    }
}
unsafe impl Send for HyprListenerCtx {}
unsafe impl Sync for HyprListenerCtx {}

static mut GLOBAL_HYPR_LISTENER_CTX: Option<HyprListenerCtx> = None;

// fn get_hypr_listener() -> MutexGuard<'static, HyprListenerCtx> {
fn get_hypr_listener() -> &'static mut HyprListenerCtx {
    unsafe {
        if GLOBAL_HYPR_LISTENER_CTX.is_none() {
            GLOBAL_HYPR_LISTENER_CTX = Some(HyprListenerCtx::new());
        }
        GLOBAL_HYPR_LISTENER_CTX.as_mut().unwrap()
    }
}

trait WorkspaceIDToInt {
    fn regular_to_i32(&self) -> Option<Result<i32, ParseIntError>>;
}
impl WorkspaceIDToInt for WorkspaceType {
    fn regular_to_i32(&self) -> Option<Result<i32, ParseIntError>> {
        match self {
            WorkspaceType::Regular(id) => Some(i32::from_str(id)),
            WorkspaceType::Special(_) => None,
        }
    }
}

pub fn init_hyprland_listener() {
    if unsafe { GLOBAL_HYPR_LISTENER_CTX.is_some() } {
        return;
    }

    enum Signal {
        Reload,
        Event(HyprEvent),
    }

    log::debug!("start init hyprland listener");

    let (s, r) = async_channel::bounded::<Signal>(1);

    let mut listener = event_listener::EventListener::new();
    {
        let s = s.clone();
        listener.add_workspace_change_handler(move |id| {
            log::debug!("received workspace change: {id}");
            if let Some(id) = id.regular_to_i32() {
                match id {
                    Ok(int) => {
                        // ignore result
                        let _ = s.send_blocking(Signal::Event(HyprEvent::Workspace(int)));
                    }
                    Err(e) => notify_hyprland_log(
                        format!("Fail to parse workspace id: {e}").as_str(),
                        false,
                    ),
                }
            }
        });
    }
    {
        let s = s.clone();
        listener.add_workspace_added_handler(move |id| {
            log::debug!("received workspace add: {id}");
            if let WorkspaceType::Regular(_) = id {
                // ignore result
                let _ = s.send_blocking(Signal::Reload);
            }
        });
    }
    {
        let s = s.clone();
        listener.add_workspace_destroy_handler(move |e| {
            log::debug!("received workspace destroy: {e:?}");
            // ignore result
            let _ = s.send_blocking(Signal::Reload);
        });
    }
    {
        let s = s.clone();
        listener.add_active_monitor_change_handler(move |e| {
            log::debug!("received monitor change: {e:?}");
            if let Some(id) = e.workspace.regular_to_i32() {
                match id {
                    Ok(int) => {
                        // ignore result
                        let _ = s.send_blocking(Signal::Event(HyprEvent::Workspace(int)));
                    }
                    Err(e) => notify_hyprland_log(
                        format!("Fail to parse workspace id: {e}").as_str(),
                        false,
                    ),
                }
            }
        });
    }

    gtk::glib::spawn_future_local(async move {
        log::info!("start hyprland workspace signal listener");
        while let Ok(s) = r.recv().await {
            match s {
                Signal::Reload => {
                    get_hypr_listener().reload();
                }
                Signal::Event(e) => {
                    get_hypr_listener().call_event(e);
                }
            }
        }
        log::info!("stop hyprland workspace signal listener");
    });

    thread::spawn(move || {
        log::info!("hyprland workspace listener is running");

        if let Err(e) = listener.start_listener() {
            notify_send("Way-Edges Hyprland error", e.to_string().as_str(), true);
            process::exit(-1)
        }

        log::info!("hyprland workspace listener stopped");
    });
}

pub fn register_hypr_event_callback(
    cb: impl FnMut(&HyprGlobalData) + 'static,
) -> (HyprCallbackId, HyprGlobalData) {
    let hypr = get_hypr_listener();
    (hypr.add_cb(Box::new(cb)), hypr.data)
}

pub fn unregister_hypr_event_callback(id: HyprCallbackId) {
    get_hypr_listener().remove_cb(id)
}

pub fn change_to_workspace(id: i32) {
    use hyprland::dispatch::*;

    log::debug!("change to workspace: {id}");

    // ignore
    let _ = Dispatch::call(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Id(
        id,
    )));
}
