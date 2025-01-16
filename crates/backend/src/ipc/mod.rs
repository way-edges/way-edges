mod listen;
pub use listen::start_ipc;

use serde::{Deserialize, Serialize};
use tokio::net::UnixStream;

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

pub async fn send_command(cmd: CommandBody) -> Result<(), Box<dyn std::error::Error>> {
    let data = serde_jsonrc::to_string(&cmd)?;
    let socket = UnixStream::connect(SOCK_FILE).await?;
    socket.writable().await?;
    socket.try_write(data.as_bytes()).map(|_| Ok(()))?
}

#[derive(Debug)]
pub enum IPCCommand {
    AddGroup(String),
    RemoveGroup(String),
    TogglePin(String, String),
    Exit,
}
