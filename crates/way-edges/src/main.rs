mod args;

use frontend::run_app;
use std::env;

fn main() {
    // completion script output, and exit
    args::if_print_completion_and_exit();

    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }
    env_logger::init();

    let cmd = args::get_args();

    match &cmd.command {
        args::Command::Daemon => {
            run_app();
        }
        _ => {
            cmd.command.send_ipc();
        }
    }
}
