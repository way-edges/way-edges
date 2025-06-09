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

    let cli = args::get_args();

    config::set_config_path(cli.config_path.as_deref());

    if let Some(cmd) = cli.command.as_ref() {
        match &cmd {
            args::Command::Daemon => {
                log::warn!("daemon command is deprecated, please just run `way-edges`");
            }
            args::Command::Schema => {
                config::output_json_schema();
                return;
            }
            _ => {
                cmd.send_ipc();
                return;
            }
        }
    }

    run_app(cli.mouse_debug);
}
