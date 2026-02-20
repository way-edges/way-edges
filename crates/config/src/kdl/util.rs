use kdl::KdlValue;

pub fn args(node: &kdl::KdlNode) -> Vec<&KdlValue> {
    node.entries()
        .iter()
        .filter_map(|entry| {
            if entry.name().is_none() {
                Some(entry.value())
            } else {
                None
            }
        })
        .collect()
}

pub fn argv(node: &kdl::KdlNode) -> &KdlValue {
    node.entry(0)
        .expect("Expected argv to be the first entry")
        .value()
}
