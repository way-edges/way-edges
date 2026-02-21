use knus::Decode;

pub mod common;
mod shared;
mod util;
pub mod widgets;

#[derive(Debug, Clone, Decode)]
pub enum TopLevelConf {
    Btn(Btn),
    Workspace(Workspace),
}

#[derive(Debug, Clone)]
pub struct Btn {
    pub common: common::CommonConfig,
    pub widget: widgets::button::BtnConfig,
}
impl<S: knus::traits::ErrorSpan> knus::Decode<S> for Btn {
    fn decode_node(
        node: &knus::ast::SpannedNode<S>,
        ctx: &mut knus::decode::Context<S>,
    ) -> Result<Self, knus::errors::DecodeError<S>> {
        Ok(Self {
            common: common::CommonConfig::decode_node(node, ctx)?,
            widget: widgets::button::BtnConfig::decode_node(node, ctx)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Workspace {
    pub common: common::CommonConfig,
    pub widget: widgets::workspace::WorkspaceConfig,
}
impl<S: knus::traits::ErrorSpan> knus::Decode<S> for Workspace {
    fn decode_node(
        node: &knus::ast::SpannedNode<S>,
        ctx: &mut knus::decode::Context<S>,
    ) -> Result<Self, knus::errors::DecodeError<S>> {
        Ok(Self {
            common: common::CommonConfig::decode_node(node, ctx)?,
            widget: widgets::workspace::WorkspaceConfig::decode_node(node, ctx)?,
        })
    }
}
