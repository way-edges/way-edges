use knus::Decode;

pub mod common;
mod shared;
mod util;
pub mod widgets;

#[derive(Debug, Clone, Decode)]
pub enum TopLevelConf {
    Btn(Btn),
    Slide(Slide),
    Workspace(Workspace),
}

macro_rules! impl_top_level_widget {
    ($name:ident, $config:ty) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            pub common: common::CommonConfig,
            pub widget: $config,
        }
        impl<S: knus::traits::ErrorSpan> knus::Decode<S> for $name {
            fn decode_node(
                node: &knus::ast::SpannedNode<S>,
                ctx: &mut knus::decode::Context<S>,
            ) -> Result<Self, knus::errors::DecodeError<S>> {
                Ok(Self {
                    common: common::CommonConfig::decode_node(node, ctx)?,
                    widget: <$config>::decode_node(node, ctx)?,
                })
            }
        }
    };
}

impl_top_level_widget!(Btn, widgets::button::BtnConfig);
impl_top_level_widget!(Slide, widgets::slide::base::SlideConfig);
impl_top_level_widget!(Workspace, widgets::workspace::WorkspaceConfig);
