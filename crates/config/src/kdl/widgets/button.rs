use crate::Widget;

pub fn parse_kdl_btn(node: &kdl::KdlNode) -> Result<Widget, String> {
    let common_config = parse_kdl_common_config(node)?;

    let mut label = None;
    let mut action = None;

    for entry in node.entries() {
        match entry.name().value() {
            "label" => {
                if let Some(value) = entry.value().as_string() {
                    label = Some(value.to_string());
                } else {
                    return Err("btn: 'label' must be a string".to_string());
                }
            }
            "action" => {
                if let Some(value) = entry.value().as_string() {
                    action = Some(value.to_string());
                } else {
                    return Err("btn: 'action' must be a string".to_string());
                }
            }
            other => {
                return Err(format!("btn: Unknown property '{other}'"));
            }
        }
    }

    let label = label.ok_or_else(|| "btn: Missing required property 'label'".to_string())?;
    let action = action.ok_or_else(|| "btn: Missing required property 'action'".to_string())?;

    Ok(crate::widgets::Widget::Button(crate::widgets::Button {
        label,
        action,
    }))
}
