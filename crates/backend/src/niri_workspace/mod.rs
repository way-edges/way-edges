mod connection;

use connection::Connection;
use niri_ipc::Workspace;
use tokio::io;

fn process_event(e: niri_ipc::Event) {
    match e {
        niri_ipc::Event::WorkspacesChanged { workspaces } => {
            println!("workspaces: {workspaces:?}")
        }
        niri_ipc::Event::WorkspaceActivated { id, focused } => {
            println!("workspace {id} activated: {focused:?}")
        }
        _ => {}
    }
}
async fn get_workspaces() -> io::Result<Vec<Workspace>> {
    let mut l = Connection::make_connection()
        .await
        .expect("Failed to connect to niri socket");

    let r = l
        .push_request(niri_ipc::Request::Workspaces)
        .await?
        .expect("Failed to open workspaces stream");

    match r {
        niri_ipc::Response::Workspaces(vec) => Ok(vec),
        _ => unreachable!(),
    }
}

async fn listen() {
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
}
