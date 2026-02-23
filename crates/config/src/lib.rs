pub mod def;
// mod serde;

use std::{
    fs::OpenOptions,
    io::Read,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use schemars::schema_for;

use crate::def::{parse_jsonc, parse_kdl, Root};

static CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn set_config_path(path: Option<&str>) {
    CONFIG_PATH
        .set(path.map(PathBuf::from).unwrap_or_else(|| {
            let bd = xdg::BaseDirectories::new();
            match bd.place_config_file("way-edges/config.jsonc") {
                Ok(p) => p,
                Err(e) => panic!("failed to create config file: {e}"),
            }
        }))
        .unwrap();
}

pub fn get_config_path() -> &'static Path {
    if CONFIG_PATH.get().is_none() {
        // If the config path is not set, we will use the default path.
        set_config_path(None);
    }

    CONFIG_PATH.get().unwrap().as_path()
}

enum ConfigContent {
    Serde(String),
    Kdl(String),
    Unknown(String),
}

fn get_config_file_content() -> Result<ConfigContent, String> {
    let p = get_config_path();

    let content = OpenOptions::new()
        .read(true)
        .open(p)
        .and_then(|mut f| {
            let mut s = String::new();
            f.read_to_string(&mut s).map(|_| s)
        })
        .map_err(|e| format!("failed to open config file: {e}"))?;

    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
        match ext {
            "kdl" => Ok(ConfigContent::Kdl(content)),
            "json" | "jsonc" => Ok(ConfigContent::Serde(content)),
            _ => {
                log::warn!("unsupported config file extension: {ext:?}");
                Ok(ConfigContent::Unknown(content))
            }
        }
    } else {
        Ok(ConfigContent::Unknown(content))
    }
}

pub fn get_config() -> Result<Root, String> {
    match get_config_file_content()? {
        ConfigContent::Serde(c) => parse_jsonc(&c),
        ConfigContent::Kdl(c) => parse_kdl(&c),
        ConfigContent::Unknown(c) => {
            // try kdl first
            parse_kdl(&c)
                .or_else(|e| {
                    log::warn!("failed to parse config file as KDL: {e}");
                    // try serde next
                    parse_jsonc(&c)
                })
                .inspect_err(|e| log::error!("failed to parse config file as KDL or JSON: {e}"))
        }
    }
}

pub fn output_json_schema() {
    let schema = schema_for!(Root);
    println!("{}", serde_jsonrc::to_string_pretty(&schema).unwrap());
}
