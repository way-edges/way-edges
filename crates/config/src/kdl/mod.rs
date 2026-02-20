mod common;
mod util;
mod widgets;

use crate::Root;

pub fn parse_kdl(content: String) -> Result<Root, String> {
    let doc = content
        .parse::<kdl::KdlDocument>()
        .map_err(|e| format!("KDL parse error: {e}"))?;

    parse_kdl_root(doc)
}

fn parse_kdl_root(doc: kdl::KdlDocument) -> Result<Root, String> {
    let mut widgets = Vec::new();

    for node in doc.nodes() {
        match node.name().value() {
            "btn" => {
                widgets.push(crate::widgets::parse_kdl_btn(node)?);
            }
            "slider" => {
                widgets.push(crate::widgets::parse_kdl_slider(node)?);
            }
            "wrap-box" => {
                widgets.push(crate::widgets::parse_kdl_wrapbox(node)?);
            }
            "workspace" => {
                widgets.push(crate::widgets::parse_kdl_workspace(node)?);
            }
            other => {
                return Err(format!("Unknown widget type: {other}"));
            }
        }
    }

    Ok(Root { widgets })
}
