use std::{
    num::ParseIntError,
    process,
    str::FromStr,
    sync::atomic::{AtomicBool, AtomicPtr},
};

use calloop::channel::Sender;
use hyprland::{
    async_closure,
    event_listener::{self},
    shared::{HyprData, HyprDataActive, WorkspaceType},
};

use util::notify_send;

use crate::{runtime::get_backend_runtime_handle, workspace::WorkspaceData};

use super::{WorkspaceCtx, WorkspaceHandler, ID};

fn notify_hyprland_log(msg: &str, is_critical: bool) {
    notify_send("Way-Edges Hyprland error", msg, is_critical);
    log::error!("{msg}");

    if is_critical {
        process::exit(-1)
    }
}

fn get_workspace_data() -> Result<WorkspaceData, String> {
    log::debug!("getting hyprland workspace data");

    let workspace_count = hyprland::data::Workspaces::get()
        .map_err(|e| e.to_string())?
        .into_iter()
        .max_by_key(|w| w.id)
        .ok_or("Failed to find available workspace")?
        .id;

    let focus = hyprland::data::Workspace::get_active()
        .map_err(|e| e.to_string())?
        .id
        - 1;

    Ok(WorkspaceData {
        workspace_count,
        focus,
    })
}

fn on_signal(s: Signal) {
    let mut call = false;
    let ctx = get_hypr_ctx();

    // NOTE: id start from 1
    match s {
        Signal::Add(id) => {
            if ctx.current.workspace_count < id {
                ctx.current = get_workspace_data().unwrap();
                call = true;
            }
        }
        Signal::Change(id) => {
            ctx.current.focus = id - 1;
            call = true;
        }
        Signal::Destroy(id) => {
            if ctx.current.workspace_count == id {
                ctx.current = get_workspace_data().unwrap();
                call = true;
            }
        }
    }
    if call {
        ctx.call();
    }
}

static CTX_INITED: AtomicBool = AtomicBool::new(false);
static GLOBAL_HYPR_LISTENER_CTX: AtomicPtr<WorkspaceCtx> = AtomicPtr::new(std::ptr::null_mut());
fn is_ctx_inited() -> bool {
    CTX_INITED.load(std::sync::atomic::Ordering::Relaxed)
}
fn get_hypr_ctx() -> &'static mut WorkspaceCtx {
    unsafe {
        GLOBAL_HYPR_LISTENER_CTX
            .load(std::sync::atomic::Ordering::Relaxed)
            .as_mut()
            .unwrap()
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

enum Signal {
    Add(i32),
    Destroy(i32),
    Change(i32),
}

fn init_hyprland_listener() {
    if is_ctx_inited() {
        return;
    }

    GLOBAL_HYPR_LISTENER_CTX.store(
        Box::into_raw(Box::new(WorkspaceCtx::new())),
        std::sync::atomic::Ordering::Relaxed,
    );
    CTX_INITED.store(true, std::sync::atomic::Ordering::Relaxed);

    let mut listener = event_listener::AsyncEventListener::new();

    listener.add_workspace_changed_handler(async_closure!(move |data| {
        let workspace_type = data.name;
        log::debug!("received workspace change: {workspace_type}");
        if let Some(id) = workspace_type.regular_to_i32() {
            match id {
                Ok(int) => {
                    on_signal(Signal::Change(int));
                }
                Err(e) => {
                    notify_hyprland_log(format!("Fail to parse workspace id: {e}").as_str(), false)
                }
            }
        }
    }));

    listener.add_workspace_added_handler(async_closure!(move |data| {
        let workspace_type = data.name;
        log::debug!("received workspace add: {workspace_type}");
        if let WorkspaceType::Regular(sid) = workspace_type {
            if let Ok(id) = i32::from_str(&sid) {
                on_signal(Signal::Add(id));
            }
        }
    }));

    listener.add_workspace_deleted_handler(async_closure!(move |e| {
        log::debug!("received workspace destroy: {e:?}");
        on_signal(Signal::Destroy(e.id));
    }));

    listener.add_active_monitor_changed_handler(async_closure!(|e| {
        log::debug!("received monitor change: {e:?}");
        if let Some(workspace_name) = e.workspace_name {
            if let Some(id) = workspace_name.regular_to_i32() {
                match id {
                    Ok(int) => {
                        on_signal(Signal::Change(int));
                    }
                    Err(e) => notify_hyprland_log(
                        format!("Fail to parse workspace id: {e}").as_str(),
                        false,
                    ),
                }
            }
        }
    }));

    get_backend_runtime_handle().spawn(async move {
        log::info!("hyprland workspace listener is running");

        if let Err(e) = listener.start_listener_async().await {
            notify_hyprland_log(e.to_string().as_str(), true);
            process::exit(-1)
        }

        log::info!("hyprland workspace listener stopped");
    });
}

pub fn register_hypr_event_callback(cb: Sender<WorkspaceData>) -> WorkspaceHandler {
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
    pub fn change_to_workspace(&mut self, workspace_id: i32) {
        use hyprland::dispatch::*;

        log::debug!("change to workspace: {workspace_id}");

        // ignore
        let _ = Dispatch::call(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Id(
            workspace_id,
        )));
    }
}
