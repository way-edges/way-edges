use serde::Deserialize;
use tokio::net::UnixStream;

use crate::{args::Command, daemon::SOCK_FILE};

#[derive(Debug, Deserialize)]
pub struct CommandBody {
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug)]
pub enum IPCCommand {
    AddGroup(String),
    RemoveGroup(String),
    Exit,
}

pub const IPC_COMMAND_ADD: &str = "add";
pub const IPC_COMMAND_REMOVE: &str = "rm";
pub const IPC_COMMAND_QUIT: &str = "q";

pub async fn send_command(cmd: &Command) -> Result<(), Box<dyn std::error::Error>> {
    let data = match cmd {
        Command::Daemon => return Ok(()),
        Command::Add { name } => {
            format!(r#"{{"command": "{IPC_COMMAND_ADD}", "args": ["{name}"]}}"#)
        }
        Command::Remove { name } => {
            format!(r#"{{"command": "{IPC_COMMAND_REMOVE}", "args": ["{name}"]}}"#)
        }
        Command::Exit => {
            format!(r#"{{"command": "{IPC_COMMAND_QUIT}"}}"#)
        }
    };

    let socket = UnixStream::connect(SOCK_FILE).await?;
    socket.writable().await?;
    socket.try_write(data.as_bytes()).map(|_| Ok(()))?
}
