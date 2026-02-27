#![allow(dead_code, unused_variables)]
use knus::Decode;
use schemars::JsonSchema;
use serde::Deserialize;

pub mod common;
pub mod shared;
mod util;
pub mod widgets;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct Root {
    pub widgets: Vec<WidgetConf>,
}

impl<S: knus::traits::ErrorSpan> knus::DecodeChildren<S> for Root {
    fn decode_children(
        nodes: &[knus::ast::SpannedNode<S>],
        ctx: &mut knus::decode::Context<S>,
    ) -> Result<Self, knus::errors::DecodeError<S>> {
        let mut widgets = vec![];
        for n in nodes {
            match n.node_name.as_ref() {
                "btn" | "slide" | "workspace" | "wrap-box" => {
                    widgets.push(WidgetConf::decode_node(n, ctx)?);
                }
                _ => {}
            }
        }

        Ok(Self { widgets })
    }
}

#[derive(Debug, Clone, Decode, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum WidgetConf {
    Btn(Btn),
    Slide(Slide),
    Workspace(Workspace),
    WrapBox(WrapBox),
}
impl WidgetConf {
    pub fn common(&self) -> &common::CommonConfig {
        match self {
            WidgetConf::Btn(c) => &c.common,
            WidgetConf::Slide(c) => &c.common,
            WidgetConf::Workspace(c) => &c.common,
            WidgetConf::WrapBox(c) => &c.common,
        }
    }
}

macro_rules! impl_top_level_widget {
    ($name:ident, $config:ty) => {
        #[derive(Debug, Clone, Deserialize, JsonSchema)]
        pub struct $name {
            #[serde(flatten)]
            pub common: common::CommonConfig,
            #[serde(flatten)]
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
impl_top_level_widget!(WrapBox, widgets::wrapbox::BoxConfig);

pub fn parse_kdl(s: &str) -> Result<Root, String> {
    match knus::parse::<Root>("config.kdl", s) {
        Ok(config) => Ok(config),
        Err(e) => {
            let msg = format!("Failed to parse kdl config: {}", e);
            println!("{:?}", miette::Report::new(e));
            Err(msg)
        }
    }
}
pub fn parse_jsonc(s: &str) -> Result<Root, String> {
    serde_jsonrc::from_str(s).map_err(|e| format!("JSON parse error: {e}"))
}
