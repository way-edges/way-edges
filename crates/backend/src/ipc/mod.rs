mod listen;
use std::{io::Write, os::unix::net::UnixStream, sync::OnceLock};

pub use listen::start_ipc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct CommandBody {
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

pub const IPC_COMMAND_RELOAD: &str = "reload";
pub const IPC_COMMAND_QUIT: &str = "q";
pub const IPC_COMMAND_TOGGLE_PIN: &str = "togglepin";

static SOCK_FILE: OnceLock<String> = OnceLock::new();

pub fn set_ipc_namespace(namespace: Option<&str>) {
    SOCK_FILE
        .set(format!(
            "/tmp/way-edges/way-edges{}.sock",
            namespace.unwrap_or_default()
        ))
        .unwrap();
}

fn get_ipc_sock() -> &'static str {
    SOCK_FILE.get().expect("IPC socket file not set")
}

pub fn send_command(cmd: CommandBody) {
    let data = serde_jsonrc::to_string(&cmd).unwrap();
    let mut socket = UnixStream::connect(get_ipc_sock()).unwrap();
    socket.write_all(data.as_bytes()).unwrap();
}

#[derive(Debug)]
pub enum IPCCommand {
    TogglePin(String),
    Reload,
    Exit,
}
