pub mod backlight;
pub mod button;
pub mod hypr_workspace;
pub mod pulseaudio;
pub mod ring;
pub mod slide;
pub mod text;
pub mod tray;
pub mod wrapbox;

mod common {
    use gtk4_layer_shell::Edge;

    use config::NumOrRelative;

    pub fn calculate_rel_extra_trigger_size(
        e: &mut NumOrRelative,
        max_size_raw: (i32, i32),
        edge: Edge,
    ) {
        if let NumOrRelative::Relative(_) = e {
            let max = match edge {
                Edge::Left | Edge::Right => max_size_raw.0,
                Edge::Top | Edge::Bottom => max_size_raw.1,
                _ => unreachable!(),
            };
            e.calculate_relative(max as f64);
        };
    }
}
