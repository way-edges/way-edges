mod draw;
mod event;
mod pre_draw;

use crate::activate::get_working_area_size;
use crate::config::{widgets::button::BtnConfig, Config, NumOrRelative};
use gtk::ApplicationWindow;
use gtk4_layer_shell::Edge;

pub fn init_widget(
    window: &ApplicationWindow,
    config: Config,
    mut btn_config: BtnConfig,
) -> Result<gtk::DrawingArea, String> {
    calculate_rel(&config, &mut btn_config)?;
    draw::setup_draw(window, config, btn_config)
}

fn calculate_rel(config: &Config, btn_config: &mut BtnConfig) -> Result<(), String> {
    if let NumOrRelative::Relative(_) = &mut btn_config.extra_trigger_size {
        let index = config.monitor.to_index()?;
        let size = get_working_area_size(index)?
            .ok_or(format!("Failed to get working area size: {index}"))?;
        let max = match config.edge {
            Edge::Left | Edge::Right => size.0,
            Edge::Top | Edge::Bottom => size.1,
            _ => unreachable!(),
        };
        btn_config.extra_trigger_size.calculate_relative(max as f64);
    };
    Ok(())
}
