pub mod conf;
mod parse;
mod raw;
pub mod widgets;

pub use conf::*;

use std::{
    fs::OpenOptions,
    io::Read,
    path::{Path, PathBuf},
    sync::OnceLock,
};

static CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();
pub fn get_config_path() -> &'static Path {
    let pb = CONFIG_PATH.get_or_init(|| {
        let bd = match xdg::BaseDirectories::new() {
            Ok(bd) => bd,
            Err(e) => panic!("no xdg base directories: {e}"),
        };

        match bd.place_config_file("way-edges/config.jsonc") {
            Ok(p) => p,
            Err(e) => panic!("failed to create config file: {e}"),
        }
    });
    let b = pb.as_path();
    b
}

fn get_config_file() -> Result<String, String> {
    let p = get_config_path();
    let mut f = match OpenOptions::new()
        // .create(true)
        .append(true)
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

pub fn get_config(group_name: Option<&str>) -> Result<GroupConfig, String> {
    let s = get_config_file()?;
    parse::parse_config(&s, group_name)
}
