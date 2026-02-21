use std::env;

use log::Level;

use crate::kdl::TopLevelConf;
use std::io::Write;

mod kdl;

fn main() {
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

    let config = match knus::parse::<Vec<TopLevelConf>>("aaa", include_str!("../debug.kdl")) {
        Ok(config) => config,
        Err(e) => {
            println!("{:?}", miette::Report::new(e));
            std::process::exit(1);
        }
    };
    println!("{:#?}", config);
}
