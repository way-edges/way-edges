use std::env;

mod activate;
mod args;
mod daemon;

// NOTE: thread 0
#[tokio::main(flavor = "current_thread")]
async fn main() {
    backend::set_main_runtime_handle();

    // completion script output, and exit
    args::if_print_completion_and_exit();

    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }

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
