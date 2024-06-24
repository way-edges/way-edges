use std::sync::OnceLock;

use clap::Parser;

/// shit
#[derive(Debug, Parser)]
#[command(name = "way-edges")]
#[command(version = "pre")]
#[command(about = "Hidden buttons on the edges", long_about = None)]
pub struct Cli {
    /// which grouop to activate
    pub group: Option<String>,

    /// whether enable mouse click output
    #[arg(short = 'd', long)]
    pub mouse_debug: bool,
}

static ARGS: OnceLock<Cli> = OnceLock::new();

pub fn get_args() -> &'static Cli {
    ARGS.get_or_init(Cli::parse)
}
