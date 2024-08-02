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
    TogglePin(String, String),
    Exit,
}

pub const IPC_COMMAND_ADD: &str = "add";
pub const IPC_COMMAND_REMOVE: &str = "rm";
pub const IPC_COMMAND_QUIT: &str = "q";
pub const IPC_COMMAND_TOGGLE_PIN: &str = "togglepin";

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
        Command::TogglePin {
            group_and_widget_name: name,
        } => {
            let (group_name, widget_name) = name
                .split_once(':')
                .ok_or("widget must be specified with: `group_name:widget_name`")?;
            format!(
                r#"{{"command": "{IPC_COMMAND_TOGGLE_PIN}", "args": ["{group_name}", "{widget_name}"]}}"#
            )
        }
    };

    let socket = UnixStream::connect(SOCK_FILE).await?;
    socket.writable().await?;
    socket.try_write(data.as_bytes()).map(|_| Ok(()))?
}
