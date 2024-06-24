use clap::Parser;

#[derive(Debug, Parser)]
pub struct Cli {
    pub group: Option<String>,
}
