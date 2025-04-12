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
    /// whether enable mouse click output, shoule be used width daemon command.
    #[arg(short = 'd', long)]
    pub mouse_debug: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

fn complete_only_group(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return vec![];
    };

    let Ok(root) = config::get_config_root() else {
        return vec![];
    };

    root.groups
        .iter()
        .filter(|group| group.name.starts_with(current))
        .map(|group| CompletionCandidate::new(&group.name))
        .collect()
}

fn complete_group_and_widget(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return vec![];
    };

    let Ok(raw_root) = config::get_config_root() else {
        return vec![];
    };

    if let Some((group_name, widget_name)) = current.split_once(':') {
        let Some(raw_group) = raw_root
            .groups
            .into_iter()
            .find(|raw_group| raw_group.name.eq(group_name))
        else {
            return vec![];
        };

        raw_group
            .widgets
            .iter()
            .filter_map(|widget| {
                let name = widget.name.as_ref()?;

                if !name.is_empty() && name.starts_with(widget_name) {
                    let name = group_name.to_owned() + ":" + name;
                    Some(CompletionCandidate::new(&name))
                } else {
                    None
                }
            })
            .collect()
    } else {
        raw_root
            .groups
            .iter()
            .filter(|raw_group| raw_group.name.starts_with(current))
            .map(|raw_group| {
                let name = raw_group.name.to_owned() + ":";
                CompletionCandidate::new(&name)
            })
            .collect()
    }
}

#[derive(Subcommand, Debug, PartialEq, Clone)]
pub enum Command {
    #[command(name = "schema")]
    Schema,

    /// run daemon. There can only be one daemon at a time.
    #[command(name = "daemon", alias = "d")]
    Daemon,

    /// add group of widgets in applicatoin given group name
    #[command(name = "add", alias = "a")]
    Add {
        /// group name
        #[clap(add = ArgValueCompleter::new(complete_only_group))]
        name: String,
    },

    /// remove group of widgets in applicatoin given group name
    #[command(name = "rm", alias = "r")]
    Remove {
        /// group name
        #[clap(add = ArgValueCompleter::new(complete_only_group))]
        name: String,
    },

    /// toggle pin of a widget under certain group.
    /// format: <group_name>:<widget_name>
    #[command(name = "togglepin")]
    TogglePin {
        /// format: <group_name>:<widget_name>
        #[clap(add = ArgValueCompleter::new(complete_group_and_widget))]
        group_and_widget_name: String,
    },

    #[command(name = "reload")]
    Reload,

    /// close daemon
    #[command(name = "quit", alias = "q")]
    Exit,
}
impl Command {
    pub fn send_ipc(&self) {
        let (command, args) = match self {
            Self::Add { name } => (ipc::IPC_COMMAND_ADD, vec![name.clone()]),
            Self::Remove { name } => (ipc::IPC_COMMAND_REMOVE, vec![name.clone()]),
            Self::Exit => (ipc::IPC_COMMAND_QUIT, vec![]),
            Self::TogglePin {
                group_and_widget_name: name,
            } => {
                let (group_name, widget_name) = name
                    .split_once(':')
                    .ok_or("widget must be specified with: `group_name:widget_name`")
                    .unwrap();
                (
                    ipc::IPC_COMMAND_TOGGLE_PIN,
                    vec![group_name.to_string(), widget_name.to_string()],
                )
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
