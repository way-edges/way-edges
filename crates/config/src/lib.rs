pub mod common;
pub mod shared;
pub mod widgets;

use schemars::{schema_for, JsonSchema};
use serde::Deserialize;

use std::{
    fs::OpenOptions,
    io::Read,
    path::{Path, PathBuf},
    sync::OnceLock,
};

pub use crate::common::CommonConfig;
pub use crate::widgets::WidgetConfig;

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

fn get_config_file_content() -> Result<String, String> {
    let p = get_config_path();

    OpenOptions::new()
        .read(true)
        .open(p)
        .and_then(|mut f| {
            let mut s = String::new();
            f.read_to_string(&mut s).map(|_| s)
        })
        .map_err(|e| format!("failed to open config file: {e}"))
}

pub fn get_config_root() -> Result<Root, String> {
    let s = get_config_file_content()?;
    serde_jsonrc::from_str(&s).map_err(|e| format!("JSON parse error: {e}"))
}

pub fn output_json_schema() {
    let schema = schema_for!(Root);
    println!("{}", serde_jsonrc::to_string_pretty(&schema).unwrap());
}

#[derive(Deserialize, Debug, JsonSchema)]
#[schemars(extend("allowTrailingCommas" = true))]
#[serde(rename_all = "kebab-case")]
pub struct Root {
    #[serde(default)]
    pub widgets: Vec<Widget>,
}

#[derive(Deserialize, Debug, JsonSchema, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Widget {
    #[serde(flatten)]
    pub common: CommonConfig,
    #[serde(flatten)]
    pub widget: WidgetConfig,
}
