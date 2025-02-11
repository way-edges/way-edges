use std::{
    collections::HashMap,
    num::ParseIntError,
    process,
    str::FromStr,
    sync::atomic::{AtomicBool, AtomicPtr},
};

use hyprland::{
    async_closure,
    data::Workspace,
    event_listener::{self},
    shared::{HyprData, HyprDataActive, WorkspaceType},
};

use crate::{runtime::get_backend_runtime_handle, workspace::WorkspaceData};

use super::{WorkspaceCB, WorkspaceCtx, WorkspaceHandler, ID};

fn sort_workspaces(v: Vec<Workspace>) -> HashMap<String, Vec<Workspace>> {
    let mut a = HashMap::new();

    v.into_iter().for_each(|f| {
        a.entry(f.monitor.clone()).or_insert(vec![]).push(f);
    });

    a.values_mut().for_each(|v| v.sort_by_key(|w| w.id));

    a
}

fn workspace_vec_to_data(v: &[Workspace], focus_id: i32) -> WorkspaceData {
    let workspace_count = v.len() as i32;
    let focus = v.iter().position(|w| w.id == focus_id).unwrap_or(0) as i32;

    WorkspaceData {
        workspace_count,
        focus,
    }
}

fn get_workspace() -> Vec<Workspace> {
    hyprland::data::Workspaces::get()
        .unwrap()
        .into_iter()
        .collect()
}

fn get_focus() -> i32 {
    hyprland::data::Workspace::get_active().unwrap().id
}

fn on_signal(s: Signal) {
    let ctx = get_hypr_ctx();

    match s {
        Signal::Change => {
            ctx.data = sort_workspaces(get_workspace());
        }
        Signal::Focus(id) => ctx.focus = id,
    }
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

// TODO: Hyprland specific config
pub struct HyprConf;

struct HyprCtx {
    workspace_ctx: WorkspaceCtx<HyprConf>,
    data: HashMap<String, Vec<Workspace>>,
    focus: i32,
}
impl HyprCtx {
    fn new() -> Self {
        Self {
            workspace_ctx: WorkspaceCtx::new(),
            data: HashMap::new(),
            focus: -1,
        }
    }
    fn get_workspace_data(
        data: &HashMap<String, Vec<Workspace>>,
        output: &str,
        focus_id: i32,
    ) -> WorkspaceData {
        let Some(wps) = data.get(output) else {
            return WorkspaceData::default();
        };
        workspace_vec_to_data(wps, focus_id)
    }
    fn call(&mut self) {
        self.workspace_ctx
            .call(|output, _| Self::get_workspace_data(&self.data, output, self.focus));
    }
    fn add_cb(&mut self, cb: WorkspaceCB<HyprConf>) -> ID {
        cb.sender
            .send(Self::get_workspace_data(&self.data, &cb.output, self.focus))
            .unwrap();
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

enum Signal {
    Change,
    Focus(i32),
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
                Ok(int) => {
                    on_signal(Signal::Focus(int));
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
            on_signal(Signal::Change);
        }
    }));

    listener.add_workspace_deleted_handler(async_closure!(move |e| {
        log::debug!("received workspace destroy: {e:?}");
        on_signal(Signal::Change);
    }));

    listener.add_active_monitor_changed_handler(async_closure!(|e| {
        log::debug!("received monitor change: {e:?}");
        if let Some(workspace_name) = e.workspace_name {
            if let Some(id) = workspace_name.regular_to_i32() {
                match id {
                    Ok(int) => {
                        on_signal(Signal::Focus(int));
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
        on_signal(Signal::Change);
        on_signal(Signal::Focus(get_focus()));
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

        let Some(id) = ctx
            .data
            .get(output)
            .and_then(|v| v.get(index))
            .map(|w| w.id)
        else {
            return;
        };

        // ignore
        let _ = Dispatch::call(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Id(
            id,
        )));
    }
}
