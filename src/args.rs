use std::sync::OnceLock;

use clap::{Parser, Subcommand};

/// shit
#[derive(Debug, Parser)]
#[command(name = "way-edges")]
#[command(version = "pre")]
#[command(about = "Hidden widget on the screen edges", long_about = None)]
pub struct Cli {
    /// whether enable mouse click output, shoule be used width daemon command.
    #[arg(short = 'd', long)]
    pub mouse_debug: bool,

    #[command(subcommand)]
    pub command: Command,
}
#[derive(Subcommand, Debug, PartialEq, Clone)]
pub enum Command {
    /// run daemon. There can only be one daemon at a time.
    #[command(name = "daemon", alias = "d")]
    Daemon,

    /// add group of widgets in applicatoin given group name
    #[command(name = "add", alias = "a")]
    Add {
        /// group name
        name: String,
    },

    /// remove group of widgets in applicatoin given group name
    #[command(name = "rm", alias = "r")]
    Remove {
        /// group name
        name: String,
    },

    /// toggle pin of a widget under certain group.
    /// format: <group_name>:<widget_name>
    #[command(name = "togglepin")]
    TogglePin {
        /// format: <group_name>:<widget_name>
        group_and_widget_name: String,
    },

    /// close daemon
    #[command(name = "quit", alias = "q")]
    Exit,
}

static ARGS: OnceLock<Cli> = OnceLock::new();

pub fn get_args() -> &'static Cli {
    ARGS.get_or_init(Cli::parse)
}
