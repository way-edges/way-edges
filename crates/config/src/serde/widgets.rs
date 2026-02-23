use schemars::JsonSchema;
use serde::Deserialize;

use crate::kdl::widgets::{
    button::BtnConfig, slide::base::SlideConfig, workspace::WorkspaceConfig, wrapbox::BoxConfig,
};

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum WidgetConfig {
    Btn(BtnConfig),
    Slider(SlideConfig),
    WrapBox(BoxConfig),
    Workspace(WorkspaceConfig),
}
