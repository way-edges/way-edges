pub mod backlight;
pub mod button;
pub mod hypr_workspace;
pub mod pulseaudio;
pub mod ring;
pub mod slide;
pub mod text;
pub mod wrapbox;

mod common {
    use gtk4_layer_shell::Edge;

    use crate::config::NumOrRelative;

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

    pub fn calculate_rel_width_height(
        w: &mut NumOrRelative,
        h: &mut NumOrRelative,
        max_size_raw: (i32, i32),
        edge: Edge,
    ) -> Result<(), String> {
        let max_size = match edge {
            Edge::Left | Edge::Right => (max_size_raw.0, max_size_raw.1),
            Edge::Top | Edge::Bottom => (max_size_raw.1, max_size_raw.0),
            _ => unreachable!(),
        };
        w.calculate_relative(max_size.0 as f64);
        h.calculate_relative(max_size.1 as f64);

        // remember to check height since we didn't do it in `parse_config`
        // when passing only `rel_height`
        let w = w.get_num()?;
        let h = h.get_num()?;
        if w * 2. > h {
            Err(format!(
                "relative height detect: width * 2 must be <= height: {w} * 2 <= {h}",
            ))
        } else {
            Ok(())
        }
    }
}
