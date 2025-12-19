mod args;

use frontend::run_app;
use log::Level;
use std::env;
use std::io::Write;

fn main() {
    // completion script output, and exit
    args::if_print_completion_and_exit();

    if env::var("RUST_LOG").is_err() {
        unsafe { env::set_var("RUST_LOG", "info,system_tray=error,zbus=warn") }
    }

    // force tracing warn
    unsafe {
        env::set_var(
            "RUST_LOG",
            format!("{},tracing=warn,usvg=error", env::var("RUST_LOG").unwrap()),
        )
    };

    eprintln!("Logging with settings: {}", env::var("RUST_LOG").unwrap());

    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            let (tag, color) = match record.level() {
                Level::Debug => ("DBG", "\x1b[90m"), // grey
                Level::Info => ("INF", "\x1b[34m"),  // blue
                Level::Warn => ("WRN", "\x1b[33m"),  // yellow
                Level::Error => ("ERR", "\x1b[31m"), // red
                Level::Trace => ("TRC", "\x1b[2m"),
            };

            writeln!(buf, "{}{}:\x1b[0m {}", color, tag, record.args())
        })
        .init();

    // env_logger::init();

    let cli = args::get_args();

    config::set_config_path(cli.config_path.as_deref());
    backend::ipc::set_ipc_namespace(cli.ipc_namespace.as_deref());

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
