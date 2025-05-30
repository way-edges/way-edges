pub mod common;
pub mod conf;
pub mod root;
pub mod widgets;

pub use conf::*;
pub use root::*;
use schemars::schema_for;

use std::{
    fs::OpenOptions,
    io::Read,
    path::{Path, PathBuf},
    // str::FromStr,
    sync::OnceLock,
};

static CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();
pub fn get_config_path() -> &'static Path {
    let pb = CONFIG_PATH.get_or_init(|| {
        let bd = xdg::BaseDirectories::new();
        match bd.place_config_file("way-edges/config.jsonc") {
            Ok(p) => p,
            Err(e) => panic!("failed to create config file: {e}"),
        }
    });
    let b = pb.as_path();
    b
}

pub fn get_config_file_content() -> Result<String, String> {
    let p = get_config_path();
    let mut f = match OpenOptions::new()
        // .create(true)
        //.append(true)
        .read(true)
        .open(p)
    {
        Ok(f) => f,
        Err(e) => return Err(format!("failed to open config file: {e}")),
    };
    let mut s = String::new();
    match f.read_to_string(&mut s) {
        Ok(_) => {}
        Err(e) => return Err(format!("failed to read config file: {e}")),
    };
    Ok(s)
}

pub fn get_config_root() -> Result<Root, String> {
    let s = get_config_file_content()?;
    root::parse_config(&s)
}

pub fn get_config_by_group(group_name: &str) -> Option<Group> {
    let mut root = get_config_root()
        .inspect_err(|e| {
            log::error!("Failed to load config: {e}");
        })
        .ok()?;

    root.take_group(group_name)
}

pub fn output_json_schema() {
    let schema = schema_for!(Root);
    println!("{}", serde_jsonrc::to_string_pretty(&schema).unwrap());
}
