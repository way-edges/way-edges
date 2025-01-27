mod connection;

use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, AtomicPtr},
};

use connection::Connection;
use niri_ipc::Workspace;
use tokio::io;

use crate::runtime::get_backend_runtime_handle;

use super::{WorkspaceCB, WorkspaceCtx, WorkspaceData, WorkspaceHandler, ID};

fn workspace_vec_to_data(v: &[Workspace]) -> WorkspaceData {
    let mut workspace_count = 0;
    let mut focus = -1;
    v.iter().enumerate().for_each(|(index, w)| {
        if w.is_focused {
            focus = index as i32;
        }
        if w.is_focused || w.active_window_id.is_some() {
            workspace_count += 1;
        }
    });

    WorkspaceData {
        workspace_count,
        focus,
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
        niri_ipc::Event::WorkspacesChanged { workspaces } => sort_workspaces(workspaces),
        niri_ipc::Event::WorkspaceActivated { id: _, focused: _ } => {
            let data = get_workspaces().await.expect("Failed to get workspaces");
            sort_workspaces(data)
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
    workspace_ctx: WorkspaceCtx,
    data: HashMap<String, Vec<Workspace>>,
}
impl NiriCtx {
    fn new() -> Self {
        Self {
            workspace_ctx: WorkspaceCtx::new(),
            data: HashMap::new(),
        }
    }
    fn get_workspace_data(data: &HashMap<String, Vec<Workspace>>, output: &str) -> WorkspaceData {
        let Some(wps) = data.get(output) else {
            return WorkspaceData::default();
        };
        workspace_vec_to_data(wps)
    }
    fn call(&mut self) {
        self.workspace_ctx
            .call(|output| Self::get_workspace_data(&self.data, output));
    }
    fn add_cb(&mut self, cb: WorkspaceCB) -> ID {
        cb.sender
            .send(Self::get_workspace_data(&self.data, &cb.output))
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
        ctx.data = sort_workspaces(wp);
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

pub fn register_niri_event_callback(cb: WorkspaceCB) -> WorkspaceHandler {
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
        let ctx = get_niri_ctx();
        let Some(output) = ctx
            .workspace_ctx
            .cb
            .get(&self.cb_id)
            .map(|w| w.output.as_str())
        else {
            return;
        };
        let Some(id) = ctx
            .data
            .get(output)
            .and_then(|v| v.get(index))
            .map(|w| w.id)
        else {
            return;
        };

        get_backend_runtime_handle().spawn(async move {
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
