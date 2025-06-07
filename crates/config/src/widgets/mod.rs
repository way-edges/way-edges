use button::BtnConfig;
use schemars::JsonSchema;
use serde::Deserialize;
use slide::base::SlideConfig;
use workspace::WorkspaceConfig;
use wrapbox::BoxConfig;

pub mod button;
pub mod slide;
pub mod workspace;
pub mod wrapbox;

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum WidgetConfig {
    Btn(BtnConfig),
    Slider(SlideConfig),
    WrapBox(BoxConfig),
    Workspace(WorkspaceConfig),
}
