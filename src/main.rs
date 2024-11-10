mod activate;
mod args;
mod config;
mod daemon;
mod file_watch;
mod ipc_command;
mod plug;
mod ui;

use std::sync::atomic::AtomicPtr;

use file_watch::*;
use ipc_command::send_command;
use notify_rust::Notification;

static MAIN_RUNTIME_HANDLE: AtomicPtr<tokio::runtime::Handle> =
    AtomicPtr::new(std::ptr::null_mut());

fn get_main_runtime_handle() -> &'static tokio::runtime::Handle {
    return unsafe {
        MAIN_RUNTIME_HANDLE
            .load(std::sync::atomic::Ordering::Acquire)
            .as_ref()
            .unwrap()
    };
}

// NOTE: thread 0
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let main_runtime_handle = tokio::runtime::Handle::current();
    MAIN_RUNTIME_HANDLE.store(
        Box::into_raw(Box::new(main_runtime_handle)),
        std::sync::atomic::Ordering::Release,
    );

    // completion script output, and exit
    args::if_print_completion_and_exit();

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
