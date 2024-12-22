mod activate;
mod args;
mod common;
mod daemon;
// mod ui;

use std::sync::atomic::AtomicPtr;

static MAIN_RUNTIME_HANDLE: AtomicPtr<tokio::runtime::Handle> =
    AtomicPtr::new(std::ptr::null_mut());

fn get_main_runtime_handle() -> &'static tokio::runtime::Handle {
    unsafe {
        MAIN_RUNTIME_HANDLE
            .load(std::sync::atomic::Ordering::Acquire)
            .as_ref()
            .unwrap()
    }
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
            cmd.command.send_ipc().await.unwrap();
        }
    }
}
