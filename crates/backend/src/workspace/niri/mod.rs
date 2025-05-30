mod connection;

use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, AtomicPtr},
};

use config::widgets::workspace::NiriConf;
use connection::Connection;
use niri_ipc::Workspace;
use tokio::io;

use crate::runtime::get_backend_runtime_handle;

use super::{WorkspaceCB, WorkspaceCtx, WorkspaceData, WorkspaceHandler, ID};

fn filter_empty_workspace(v: &[Workspace]) -> Vec<&Workspace> {
    v.iter()
        .filter(|w| w.is_focused || w.active_window_id.is_some())
        .collect()
}

#[derive(Default)]
struct DataCache {
    inner: HashMap<String, Vec<Workspace>>,
    focused_output: Option<String>,
}
impl DataCache {
    fn new(map: HashMap<String, Vec<Workspace>>) -> Self {
        // Determine which output is focused by looking for the focused workspace
        let focused_output = map.iter()
            .find_map(|(output_name, workspaces)| {
                if workspaces.iter().any(|w| w.is_focused) {
                    Some(output_name.clone())
                } else {
                    None
                }
            });
        
        Self { 
            inner: map,
            focused_output,
        }
    }
    fn get_workspace_data(&self, output: &str, filter_empty: bool) -> WorkspaceData {
        let Some(wps) = self.inner.get(output) else {
            return WorkspaceData::default();
        };

        let v = if filter_empty {
            filter_empty_workspace(wps)
        } else {
            wps.iter().collect()
        };
        let focus = v
            .iter()
            .position(|w| w.is_focused)
            .map(|i| i as i32)
            .unwrap_or(-1);
        let active = v
            .iter()
            .position(|w| w.is_active)
            .map(|i| i as i32)
            .unwrap_or(-1);
        let workspace_count = v.len() as i32;

        WorkspaceData {
            workspace_count,
            focus,
            active,
        }
    }
    fn get_workspace(&self, output: &str, filter_empty: bool, index: usize) -> Option<&Workspace> {
        let wps = self.inner.get(output)?;

        let v = if filter_empty {
            filter_empty_workspace(wps)
        } else {
            wps.iter().collect()
        };
        if v.len() > index {
            Some(v[index])
        } else {
            None
        }
    }
}

fn sort_workspaces(v: Vec<Workspace>) -> HashMap<String, Vec<Workspace>> {
    let mut a = HashMap::new();

    v.into_iter().for_each(|mut f| {
        let Some(o) = f.output.take() else {
            return;
        };
        a.entry(o).or_insert(vec![]).push(f);
    });

    a.values_mut().for_each(|v| v.sort_by_key(|w| w.idx));

    a
}

async fn process_event(e: niri_ipc::Event) {
    log::debug!("niri event: {e:?}");

    let ctx = get_niri_ctx();
    // NOTE: id start from 1
    ctx.data = match e {
        niri_ipc::Event::WorkspaceActivated { id: _, focused: _ } => {
            let data = get_workspaces().await.expect("Failed to get workspaces");
            DataCache::new(sort_workspaces(data))
        }
        niri_ipc::Event::WorkspacesChanged { workspaces } => {
            DataCache::new(sort_workspaces(workspaces))
        }
        _ => {
            return;
        }
    };

    ctx.call();
}
async fn get_workspaces() -> io::Result<Vec<Workspace>> {
    let mut l = Connection::make_connection()
        .await
        .expect("Failed to connect to niri socket");

    let r = l
        .push_request(niri_ipc::Request::Workspaces)
        .await?
        .expect("Failed to request workspaces");

    match r {
        niri_ipc::Response::Workspaces(vec) => Ok(vec),
        _ => unreachable!(),
    }
}

static CTX_INITED: AtomicBool = AtomicBool::new(false);
static GLOBAL_NIRI_LISTENER_CTX: AtomicPtr<NiriCtx> = AtomicPtr::new(std::ptr::null_mut());
fn is_ctx_inited() -> bool {
    CTX_INITED.load(std::sync::atomic::Ordering::Relaxed)
}
fn get_niri_ctx() -> &'static mut NiriCtx {
    unsafe {
        GLOBAL_NIRI_LISTENER_CTX
            .load(std::sync::atomic::Ordering::Relaxed)
            .as_mut()
            .unwrap()
    }
}

struct NiriCtx {
    workspace_ctx: WorkspaceCtx<NiriConf>,
    data: DataCache,
}
impl NiriCtx {
    fn new() -> Self {
        Self {
            workspace_ctx: WorkspaceCtx::new(),
            data: DataCache::default(),
        }
    }
    fn call(&mut self) {
        self.workspace_ctx
            .call(|output, conf, focused_only| {
                // If focused_only is enabled, only send updates to the focused monitor
                if focused_only {
                    if let Some(ref focused_output) = self.data.focused_output {
                        if output != focused_output {
                            return None; // Skip this monitor
                        }
                    } else {
                        return None; // No focused monitor found, skip all
                    }
                }
                Some(self.data.get_workspace_data(output, conf.filter_empty))
            });
    }
    fn add_cb(&mut self, cb: WorkspaceCB<NiriConf>) -> ID {
        cb.sender
            .send(
                self.data
                    .get_workspace_data(&cb.output, cb.data.filter_empty),
            )
            .unwrap();
        self.workspace_ctx.add_cb(cb)
    }
    fn remove_cb(&mut self, id: ID) {
        self.workspace_ctx.remove_cb(id);
    }
}

fn start_listener() {
    if is_ctx_inited() {
        return;
    }

    GLOBAL_NIRI_LISTENER_CTX.store(
        Box::into_raw(Box::new(NiriCtx::new())),
        std::sync::atomic::Ordering::Relaxed,
    );
    CTX_INITED.store(true, std::sync::atomic::Ordering::Relaxed);
    get_backend_runtime_handle().spawn(async {
        let wp = get_workspaces().await.expect("Failed to get workspaces");
        let ctx = get_niri_ctx();
        ctx.data = DataCache::new(sort_workspaces(wp));
        ctx.call();
    });

    get_backend_runtime_handle().spawn(async {
        let mut l = Connection::make_connection()
            .await
            .expect("Failed to connect to niri socket")
            .to_listener()
            .await
            .expect("Failed to send EventStream request");

        let mut buf = String::new();
        loop {
            match l.next_event(&mut buf).await {
                Ok(e) => process_event(e).await,
                Err(err) => {
                    log::error!("error reading from event stream: {}", err);
                    break;
                }
            }
            buf.clear();
        }
        log::error!("niri event stream closed")
    });
}

pub fn register_niri_event_callback(cb: WorkspaceCB<NiriConf>) -> WorkspaceHandler {
    start_listener();
    let cb_id = get_niri_ctx().add_cb(cb);
    WorkspaceHandler::Niri(NiriWorkspaceHandler { cb_id })
}

pub fn unregister_niri_event_callback(id: ID) {
    get_niri_ctx().remove_cb(id)
}

#[derive(Debug)]
pub struct NiriWorkspaceHandler {
    cb_id: ID,
}
impl Drop for NiriWorkspaceHandler {
    fn drop(&mut self) {
        unregister_niri_event_callback(self.cb_id);
    }
}
impl NiriWorkspaceHandler {
    pub fn change_to_workspace(&mut self, index: usize) {
        let cb_id = self.cb_id;
        get_backend_runtime_handle().spawn(async move {
            let ctx = get_niri_ctx();

            let Some((output, filter_empty)) = ctx
                .workspace_ctx
                .cb
                .get(&cb_id)
                .map(|w| (w.output.as_str(), w.data.filter_empty))
            else {
                return;
            };

            let Some(id) = ctx
                .data
                .get_workspace(output, filter_empty, index)
                .map(|w| w.id)
            else {
                return;
            };

            connection::Connection::make_connection()
                .await
                .expect("Failed to connect to niri socket")
                .push_request(niri_ipc::Request::Action(
                    niri_ipc::Action::FocusWorkspace {
                        reference: niri_ipc::WorkspaceReferenceArg::Id(id),
                    },
                ))
                .await
                .expect("Failed to request workspace change")
                .expect("request error");
        });
    }
}
