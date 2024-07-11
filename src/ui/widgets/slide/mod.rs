mod draw;
mod event;
mod pre_draw;

use std::{cell::Cell, rc::Weak};

use crate::{
    activate::get_monior_size,
    config::{widgets::slide::SlideConfig, Config},
};
use gio::glib::WeakRef;
use gtk::ApplicationWindow;

use super::common;

pub struct SlideExpose {
    pub darea: WeakRef<gtk::DrawingArea>,
    pub progress: Weak<Cell<f64>>,
}

pub fn init_widget(
    window: &ApplicationWindow,
    config: Config,
    mut slide_cfg: SlideConfig,
) -> Result<SlideExpose, String> {
    calculate_rel(&config, &mut slide_cfg)?;
    draw::setup_draw(window, config, slide_cfg)
}

fn calculate_rel(config: &Config, slide_config: &mut SlideConfig) -> Result<(), String> {
    let index = config.monitor.to_index()?;
    let size =
        // get_working_area_size(index)?.ok_or(format!("Failed to get working area size: {index}"))?;
        get_monior_size(index)?.ok_or(format!("Failed to get working area size: {index}"))?;

    common::calculate_rel_extra_trigger_size(
        &mut slide_config.extra_trigger_size,
        size,
        config.edge,
    );

    common::calculate_rel_width_height(
        &mut slide_config.width,
        &mut slide_config.height,
        size,
        config.edge,
    )?;
    Ok(())
}
