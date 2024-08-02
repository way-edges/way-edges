mod activate;
mod args;
mod config;
mod daemon;
mod file_watch;
mod ipc_command;
mod plug;
mod ui;

use file_watch::*;
use ipc_command::send_command;
use notify_rust::Notification;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();

    // for cmd line help msg.
    // or else it will show help from `gtk` other than `clap`
    let cmd = args::get_args();
    match &cmd.command {
        args::Command::Daemon => {
            daemon::daemon().await;
        }
        _ => {
            send_command(&cmd.command).await.unwrap();
        }
    }
}

pub fn notify_send(summary: &str, body: &str, is_critical: bool) {
    let mut n = Notification::new();
    n.summary(summary);
    n.body(body);
    if is_critical {
        n.urgency(notify_rust::Urgency::Critical);
    }
    if let Err(e) = n.show() {
        log::error!("Failed to send notification: \"{summary}\" - \"{body}\"\nError: {e}");
    }
}
