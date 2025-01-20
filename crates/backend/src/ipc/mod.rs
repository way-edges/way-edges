mod listen;
use std::{io::Write, os::unix::net::UnixStream};

pub use listen::start_ipc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct CommandBody {
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

pub const IPC_COMMAND_ADD: &str = "add";
pub const IPC_COMMAND_REMOVE: &str = "rm";
pub const IPC_COMMAND_QUIT: &str = "q";
pub const IPC_COMMAND_TOGGLE_PIN: &str = "togglepin";

pub const SOCK_FILE: &str = "/tmp/way-edges/way-edges.sock";

pub fn send_command(cmd: CommandBody) {
    let data = serde_jsonrc::to_string(&cmd).unwrap();
    let mut socket = UnixStream::connect(SOCK_FILE).unwrap();
    socket.write_all(data.as_bytes()).unwrap();
}

#[derive(Debug)]
pub enum IPCCommand {
    AddGroup(String),
    RemoveGroup(String),
    TogglePin(String, String),
    Exit,
}
