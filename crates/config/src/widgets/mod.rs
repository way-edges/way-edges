use button::BtnConfig;
use schemars::JsonSchema;
use serde::Deserialize;
use slide::base::SlideConfig;
use workspace::WorkspaceConfig;
use wrapbox::BoxConfig;

use crate::common::CommonConfig;

pub mod button;
pub mod slide;
pub mod workspace;
pub mod wrapbox;

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum Widget {
    Btn(BtnConfig),
    Slider(SlideConfig),
    WrapBox(BoxConfig),
    Workspace(WorkspaceConfig),
}

impl Widget {
    pub fn get_common_config(&self) -> &CommonConfig {
        match self {
            Widget::Btn(config) => &config.common,
            Widget::Slider(config) => &config.common,
            Widget::WrapBox(config) => &config.common,
            Widget::Workspace(config) => &config.common,
        }
    }
    pub fn get_common_config_mut(&mut self) -> &mut CommonConfig {
        match self {
            Widget::Btn(config) => &mut config.common,
            Widget::Slider(config) => &mut config.common,
            Widget::WrapBox(config) => &mut config.common,
            Widget::Workspace(config) => &mut config.common,
        }
    }
}
