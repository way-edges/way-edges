use std::{
    collections::HashMap,
    num::ParseIntError,
    process,
    str::FromStr,
    sync::atomic::{AtomicBool, AtomicPtr},
};

use hyprland::{
    async_closure,
    data::{Monitor, Workspace},
    event_listener::{self},
    shared::{HyprData, HyprDataActive, WorkspaceType},
};

use crate::{runtime::get_backend_runtime_handle, workspace::WorkspaceData};

use super::{WorkspaceCB, WorkspaceCtx, WorkspaceHandler, ID};

fn sort_workspaces(v: Vec<Workspace>, m: Vec<Monitor>) -> HashMap<String, (Vec<Workspace>, i32)> {
    let mut map = HashMap::new();

    v.into_iter().for_each(|f| {
        map.entry(f.monitor.clone())
            .or_insert((vec![], 0))
            .0
            .push(f);
    });

    map.iter_mut().for_each(|(k, (v, active))| {
        v.retain(|w| w.id > 0); // common workspaces id start from 1
        v.sort_by_key(|w| w.id);

        *active = m
            .iter()
            .find(|m| &m.name == k)
            .map(|monitor| monitor.active_workspace.id)
            .unwrap_or(-1);
    });

    map
}

fn workspace_vec_to_data(v: &[Workspace], focus_id: i32, active: i32) -> WorkspaceData {
    // Count actual workspaces on this monitor, not assuming contiguous IDs
    let min_id = v.first().map(|w| w.id).unwrap();
    let max_id = v.last().map(|w| w.id).unwrap();
    let workspace_count = max_id - min_id + 1;

    let active = active - min_id;
    let focus = if focus_id < min_id || focus_id > max_id {
        -1
    } else {
        active
    };

    WorkspaceData {
        workspace_count,
        focus,
        active,
    }
}

fn get_workspace() -> Vec<Workspace> {
    hyprland::data::Workspaces::get()
        .unwrap()
        .into_iter()
        .collect()
}

fn get_monitors() -> Vec<Monitor> {
    hyprland::data::Monitors::get()
        .unwrap()
        .into_iter()
        .collect()
}

fn get_focus() -> i32 {
    hyprland::data::Workspace::get_active().unwrap().id
}

fn get_focused_monitor() -> Option<String> {
    hyprland::data::Monitors::get()
        .ok()?
        .into_iter()
        .find(|m| m.focused)
        .map(|m| m.name)
}

fn on_signal() {
    let ctx = get_hypr_ctx();
    let is_initial_sync = ctx.data.is_default();

    ctx.data.map = sort_workspaces(get_workspace(), get_monitors());
    ctx.data.focus = get_focus();
    // Always try to update focused_monitor after fetching new data
    ctx.data.focused_monitor = get_focused_monitor();

    if is_initial_sync {
        // Perform the unconditional sync only on the first population
        ctx.workspace_ctx
            .sync_all_widgets_unconditionally(|output, _conf_data| {
                ctx.data.get_workspace_data(output)
            });
    }

    // Regular call that respects focused_only
    ctx.call();
}

fn on_monitor_focus_change(monitor_name: &str) {
    let ctx = get_hypr_ctx();
    ctx.data.focused_monitor = Some(monitor_name.to_string());
    log::debug!("Monitor focus changed to: {}", monitor_name);
    ctx.call();
}

static CTX_INITED: AtomicBool = AtomicBool::new(false);
static GLOBAL_HYPR_LISTENER_CTX: AtomicPtr<HyprCtx> = AtomicPtr::new(std::ptr::null_mut());
fn is_ctx_inited() -> bool {
    CTX_INITED.load(std::sync::atomic::Ordering::Relaxed)
}
fn get_hypr_ctx() -> &'static mut HyprCtx {
    unsafe {
        GLOBAL_HYPR_LISTENER_CTX
            .load(std::sync::atomic::Ordering::Relaxed)
            .as_mut()
            .unwrap()
    }
}

#[derive(Debug)]
struct CacheData {
    // workspace and active id
    map: HashMap<String, (Vec<Workspace>, i32)>,
    focus: i32,
    // Track the currently focused monitor for focused_only feature
    focused_monitor: Option<String>,
}
impl CacheData {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            focus: -1,
            focused_monitor: None,
        }
    }

    fn is_default(&self) -> bool {
        self.map.is_empty() && self.focus == -1 && self.focused_monitor.is_none()
    }

    fn get_workspace_data(&self, output: &str) -> WorkspaceData {
        let Some((wps, active)) = self.map.get(output) else {
            return WorkspaceData::default();
        };
        workspace_vec_to_data(wps, self.focus, *active)
    }
}

// TODO: Hyprland specific config
pub struct HyprConf;

struct HyprCtx {
    workspace_ctx: WorkspaceCtx<HyprConf>,
    data: CacheData,
}
impl HyprCtx {
    fn new() -> Self {
        Self {
            workspace_ctx: WorkspaceCtx::new(),
            data: CacheData::new(),
        }
    }
    fn call(&mut self) {
        self.workspace_ctx.call(|output, _, focused_only| {
            if focused_only {
                // Only send updates to the currently focused monitor
                if let Some(ref focused_monitor) = self.data.focused_monitor {
                    if output != focused_monitor {
                        return None; // Skip non-focused monitors
                    }
                } else {
                    return None; // No focused monitor known yet
                }
            }
            Some(self.data.get_workspace_data(output))
        });
    }
    fn add_cb(&mut self, cb: WorkspaceCB<HyprConf>) -> ID {
        if !self.data.is_default() {
            cb.sender
                .send(self.data.get_workspace_data(&cb.output))
                .unwrap_or_else(|e| log::error!("Error sending initial data in add_cb: {}", e));
        }
        self.workspace_ctx.add_cb(cb)
    }
    fn remove_cb(&mut self, id: ID) {
        self.workspace_ctx.remove_cb(id);
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

fn init_hyprland_listener() {
    if is_ctx_inited() {
        return;
    }

    GLOBAL_HYPR_LISTENER_CTX.store(
        Box::into_raw(Box::new(HyprCtx::new())),
        std::sync::atomic::Ordering::Relaxed,
    );
    CTX_INITED.store(true, std::sync::atomic::Ordering::Relaxed);

    let mut listener = event_listener::AsyncEventListener::new();

    listener.add_workspace_changed_handler(async_closure!(move |data| {
        let workspace_type = data.name;
        log::debug!("received workspace change: {workspace_type}");
        if let Some(id) = workspace_type.regular_to_i32() {
            match id {
                Ok(_) => {
                    on_signal();
                }
                Err(e) => {
                    log::error!("Fail to parse workspace id: {e}");
                }
            }
        }
    }));

    listener.add_workspace_added_handler(async_closure!(move |data| {
        let workspace_type = data.name;
        log::debug!("received workspace add: {workspace_type}");
        if let WorkspaceType::Regular(_) = workspace_type {
            on_signal();
        }
    }));

    listener.add_workspace_deleted_handler(async_closure!(move |e| {
        log::debug!("received workspace destroy: {e:?}");
        on_signal();
    }));

    listener.add_active_monitor_changed_handler(async_closure!(|e| {
        log::debug!("received monitor change: {e:?}");

        // Update focused monitor for focused_only feature
        on_monitor_focus_change(&e.monitor_name);

        if let Some(workspace_name) = e.workspace_name {
            if let Some(id) = workspace_name.regular_to_i32() {
                match id {
                    Ok(_) => {
                        on_signal();
                    }
                    Err(e) => log::error!("Fail to parse workspace id: {e}"),
                }
            }
        }
    }));

    get_backend_runtime_handle().spawn(async move {
        log::info!("hyprland workspace listener is running");

        if let Err(e) = listener.start_listener_async().await {
            log::error!("{e}");
            process::exit(-1)
        }

        log::info!("hyprland workspace listener stopped");
    });

    get_backend_runtime_handle().spawn(async {
        on_signal();
    });
}

pub fn register_hypr_event_callback(cb: WorkspaceCB<HyprConf>) -> WorkspaceHandler {
    init_hyprland_listener();
    let cb_id = get_hypr_ctx().add_cb(cb);
    WorkspaceHandler::Hyprland(HyprWorkspaceHandler { cb_id })
}

pub fn unregister_hypr_event_callback(id: ID) {
    get_hypr_ctx().remove_cb(id)
}

#[derive(Debug)]
pub struct HyprWorkspaceHandler {
    cb_id: ID,
}
impl Drop for HyprWorkspaceHandler {
    fn drop(&mut self) {
        unregister_hypr_event_callback(self.cb_id);
    }
}
impl HyprWorkspaceHandler {
    pub fn change_to_workspace(&mut self, index: usize) {
        use hyprland::dispatch::*;

        let ctx = get_hypr_ctx();
        let Some(output) = ctx
            .workspace_ctx
            .cb
            .get(&self.cb_id)
            .map(|w| w.output.as_str())
        else {
            return;
        };

        log::debug!("change to workspace: {output} - {index}");

        let Some(id) = ctx.data.map.get(output).and_then(|(v, _)| {
            // Use the actual workspace ID at the given index
            v.first().map(|w| w.id + index as i32)
        }) else {
            return;
        };

        // ignore
        let _ = Dispatch::call(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Id(
            id,
        )));
    }
}
