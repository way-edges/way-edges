use std::sync::OnceLock;

use clap::{CommandFactory, Parser, Subcommand};

use clap_complete::{
    engine::{ArgValueCompleter, CompletionCandidate},
    CompleteEnv,
};

use crate::config;

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
    pub command: Command,
}

fn complete_only_group(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return vec![];
    };

    let Ok(raw_root) = config::get_config_raw() else {
        return vec![];
    };

    raw_root
        .groups
        .iter()
        .filter(|raw_group| raw_group.name.starts_with(current))
        .map(|raw_group| CompletionCandidate::new(&raw_group.name))
        .collect()
}

fn complete_group_and_widget(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return vec![];
    };

    let Ok(raw_root) = config::get_config_raw() else {
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
            .filter(|widget| !widget.name.is_empty() && widget.name.starts_with(widget_name))
            .map(|widget| {
                let name = group_name.to_owned() + ":" + &widget.name;
                CompletionCandidate::new(&name)
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

    /// close daemon
    #[command(name = "quit", alias = "q")]
    Exit,
}

static ARGS: OnceLock<Cli> = OnceLock::new();

pub fn get_args() -> &'static Cli {
    ARGS.get_or_init(Cli::parse)
}

/// nothing should be printed to stdout before this.
pub fn if_print_completion_and_exit() {
    CompleteEnv::with_factory(Cli::command).complete();
}
