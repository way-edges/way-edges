use super::{CommandBody, IPCCommand};
use crate::{
    IPC_COMMAND_ADD, IPC_COMMAND_QUIT, IPC_COMMAND_REMOVE, IPC_COMMAND_TOGGLE_PIN, SOCK_FILE,
};
use std::{future::Future, path::Path};

use tokio::net::UnixStream;

use util::notify_send;

pub async fn listen_ipc(
    mut on_command: impl FnMut(IPCCommand),
) -> (tokio::task::JoinHandle<()>, impl Future<Output = ()>) {
    let (ipc_command_sender, ipc_command_receiver) = async_channel::unbounded::<IPCCommand>();

    let processer = async move {
        while let Ok(command) = ipc_command_receiver.recv().await {
            on_command(command)
        }
    };

    let listener = {
        let path = Path::new(SOCK_FILE);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let _ = std::fs::remove_file(SOCK_FILE);
        tokio::net::UnixListener::bind(SOCK_FILE).unwrap()
    };

    let ipc_task = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    deal_stream_in_background(stream, ipc_command_sender.clone());
                }
                Err(e) => {
                    let msg = format!("Fail to connect socket: {e}");
                    notify_send("Way-edges", msg.as_str(), true);
                    log::error!("msg");
                    break;
                }
            }
        }
    });

    (ipc_task, processer)
}

fn deal_stream_in_background(stream: UnixStream, sender: async_channel::Sender<IPCCommand>) {
    tokio::spawn(async move {
        let raw = stream_read_all(&stream).await?;
        log::debug!("recv ipc msg: {raw}");
        let command_body =
            serde_jsonrc::from_str::<CommandBody>(&raw).map_err(|e| e.to_string())?;
        let ipc = match command_body.command.as_str() {
            IPC_COMMAND_ADD => {
                IPCCommand::AddGroup(command_body.args.first().ok_or("No group name")?.clone())
            }
            IPC_COMMAND_REMOVE => {
                IPCCommand::RemoveGroup(command_body.args.first().ok_or("No group name")?.clone())
            }
            IPC_COMMAND_TOGGLE_PIN => IPCCommand::TogglePin(
                command_body.args.first().ok_or("No group name")?.clone(),
                command_body.args.get(1).ok_or("No widget name")?.clone(),
            ),
            IPC_COMMAND_QUIT => IPCCommand::Exit,
            _ => return Err("unknown command".to_string()),
        };
        sender
            .send(ipc)
            .await
            .map_err(|_| "ipc channel closed".to_string())?;
        Ok(())
    });
}

async fn stream_read_all(stream: &UnixStream) -> Result<String, String> {
    let mut buf_array = vec![];
    let a = loop {
        // Wait for the socket to be readable
        if stream.readable().await.is_err() {
            return Err("stream not readable".to_string());
        }

        // Creating the buffer **after** the `await` prevents it from
        // being stored in the async task.
        let mut buf = [0; 4096];

        // Try to read data, this may still fail with `WouldBlock`
        // if the readiness event is a false positive.
        match stream.try_read(&mut buf) {
            Ok(0) => break String::from_utf8_lossy(&buf_array),
            Ok(n) => {
                buf_array.extend_from_slice(&buf[..n]);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                return Err(format!("Can not read command: {e}"));
            }
        }
    };

    Ok(a.to_string())
}
