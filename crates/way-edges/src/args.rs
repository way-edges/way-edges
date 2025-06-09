use std::sync::OnceLock;

use backend::ipc;
use clap::{CommandFactory, Parser, Subcommand};

use clap_complete::{
    engine::{ArgValueCompleter, CompletionCandidate},
    CompleteEnv,
};

#[derive(Debug, Parser)]
#[command(name = "way-edges")]
#[command(author = "OGIOS")]
#[command(version = "pre")]
#[command(about = "Hidden widget on the screen edges", long_about = None)]
pub struct Cli {
    /// print the mouse button key to the log when press and release.
    #[arg(short = 'd', long)]
    pub mouse_debug: bool,

    #[arg(short = 'c', long)]
    pub config_path: Option<String>,

    #[arg(short = 'i', long)]
    pub ipc_namespace: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

fn complete_widget_name(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return vec![];
    };

    let Ok(root) = config::get_config_root() else {
        return vec![];
    };

    root.widgets
        .into_iter()
        .filter(|w| w.common.namespace.starts_with(current))
        .map(|w| CompletionCandidate::new(&w.common.namespace))
        .collect()
}

#[derive(Subcommand, Debug, PartialEq, Clone)]
pub enum Command {
    /// print json schema of the configurations to the stdout
    #[command(name = "schema")]
    Schema,

    /// (deprecated) run daemon. There can only be one daemon at a time.
    #[command(name = "daemon", alias = "d")]
    Daemon,

    /// toggle pin of a widget under certain group.
    /// format: <group_name>:<widget_name>
    #[command(name = "togglepin")]
    TogglePin {
        /// format: <group_name>:<widget_name>
        #[clap(add = ArgValueCompleter::new(complete_widget_name))]
        namespace: String,
    },

    /// reload widget configuration
    #[command(name = "reload")]
    Reload,

    /// close daemon
    #[command(name = "quit", alias = "q")]
    Exit,
}
impl Command {
    pub fn send_ipc(&self) {
        let (command, args) = match self {
            Self::Exit => (ipc::IPC_COMMAND_QUIT, vec![]),
            Self::TogglePin { namespace } => {
                (ipc::IPC_COMMAND_TOGGLE_PIN, vec![namespace.to_string()])
            }
            Self::Reload => (ipc::IPC_COMMAND_RELOAD, vec![]),
            _ => {
                return;
            }
        };

        ipc::send_command(ipc::CommandBody {
            command: command.to_string(),
            args,
        });
    }
}

static ARGS: OnceLock<Cli> = OnceLock::new();

pub fn get_args() -> &'static Cli {
    ARGS.get_or_init(Cli::parse)
}

/// nothing should be printed to stdout before this.
pub fn if_print_completion_and_exit() {
    CompleteEnv::with_factory(Cli::command).complete();
}
