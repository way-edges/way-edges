mod connection;

use std::sync::atomic::{AtomicBool, AtomicPtr};

use calloop::channel::Sender;
use connection::Connection;
use niri_ipc::Workspace;
use tokio::io;

use crate::runtime::get_backend_runtime_handle;

use super::{WorkspaceCtx, WorkspaceData, ID};

fn workspace_vec_to_data(v: Vec<Workspace>) -> WorkspaceData {
    // TODO: FILTER OUT THE EMPTY WORKSPACE IN THE END

    let workspace_count = v.len() as i32;
    let focus = v.iter().position(|w| w.is_focused).unwrap_or(0) as i32;

    WorkspaceData {
        workspace_count,
        focus,
    }
}

fn process_event(e: niri_ipc::Event) {
    log::debug!("niri event: {e:?}");

    let ctx = get_niri_ctx();
    // NOTE: id start from 1
    match e {
        niri_ipc::Event::WorkspacesChanged { workspaces } => {
            ctx.current = workspace_vec_to_data(workspaces);
        }
        niri_ipc::Event::WorkspaceActivated { id, focused } => {
            if id > ctx.current.workspace_count as u64 && focused {
                ctx.current.workspace_count = id as i32;
                ctx.current.focus = id as i32 - 1;
            } else if focused {
                ctx.current.focus = id as i32 - 1;
            }
        }
        _ => {
            return;
        }
    }
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
static GLOBAL_NIRI_LISTENER_CTX: AtomicPtr<WorkspaceCtx> = AtomicPtr::new(std::ptr::null_mut());
fn is_ctx_inited() -> bool {
    CTX_INITED.load(std::sync::atomic::Ordering::Relaxed)
}
fn get_niri_ctx() -> &'static mut WorkspaceCtx {
    unsafe {
        GLOBAL_NIRI_LISTENER_CTX
            .load(std::sync::atomic::Ordering::Relaxed)
            .as_mut()
            .unwrap()
    }
}

fn start_listener() {
    if is_ctx_inited() {
        return;
    }

    GLOBAL_NIRI_LISTENER_CTX.store(
        Box::into_raw(Box::new(WorkspaceCtx::new())),
        std::sync::atomic::Ordering::Relaxed,
    );
    CTX_INITED.store(true, std::sync::atomic::Ordering::Relaxed);
    get_backend_runtime_handle().spawn(async {
        let wp = get_workspaces().await.expect("Failed to get workspaces");
        let ctx = get_niri_ctx();
        ctx.current = workspace_vec_to_data(wp);
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
                Ok(e) => process_event(e),
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

pub fn register_niri_event_callback(cb: Sender<WorkspaceData>) -> ID {
    start_listener();
    get_niri_ctx().add_cb(cb)
}

pub fn unregister_niri_event_callback(id: ID) {
    get_niri_ctx().remove_cb(id)
}
