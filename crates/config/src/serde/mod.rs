pub mod widgets;

use schemars::{schema_for, JsonSchema};
use serde::Deserialize;

use std::{
    fs::OpenOptions,
    io::Read,
    path::{Path, PathBuf},
    sync::OnceLock,
};

pub use crate::kdl::common::CommonConfig;
pub use crate::serde::widgets::WidgetConfig;

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
