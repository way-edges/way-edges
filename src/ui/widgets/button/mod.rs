mod draw;
mod event;
mod pre_draw;

use std::time::Duration;

use crate::activate::monitor::get_monitor_context;
use crate::config::{widgets::button::BtnConfig, Config, NumOrRelative};
use crate::ui::draws::transition_state::TransitionStateList;
use crate::ui::{WidgetExpose, WidgetExposePtr};
use gtk::prelude::{GtkWindowExt, WidgetExt};
use gtk::ApplicationWindow;
use gtk::DrawingArea;
use gtk4_layer_shell::Edge;

struct ButtonWidgetExpose;
impl WidgetExpose for ButtonWidgetExpose {}

pub fn init_widget(
    window: &ApplicationWindow,
    config: Config,
    mut btn_config: BtnConfig,
) -> Result<WidgetExposePtr, String> {
    calculate_rel(&config, &mut btn_config)?;

    let darea = DrawingArea::new();
    window.set_child(Some(&darea));
    let size = &btn_config.size;
    let edge = config.edge;
    let map_size = (
        size.thickness.get_num().unwrap() as i32,
        size.length.get_num().unwrap() as i32,
    );

    // set widget size
    match edge {
        Edge::Left | Edge::Right => {
            darea.set_width_request(map_size.0);
            darea.set_height_request(map_size.1);
        }
        Edge::Top | Edge::Bottom => {
            darea.set_width_request(map_size.1);
            darea.set_height_request(map_size.0);
        }
        _ => unreachable!(),
    };

    // visible range is 0 -> width
    let mut ts_list = TransitionStateList::new();
    let pop_ts = ts_list
        .new_transition(Duration::from_millis(btn_config.transition_duration))
        .item;
    let ms = event::setup_event(
        &darea,
        btn_config.event_map.take().ok_or("EventMap is None")?,
        pop_ts.clone(),
    );

    draw::setup_draw(window, &darea, config, btn_config, ms, ts_list, pop_ts)?;

    Ok(Box::new(ButtonWidgetExpose))
}

fn calculate_rel(config: &Config, btn_config: &mut BtnConfig) -> Result<(), String> {
    let monitor_size = get_monitor_context()
        .get_monitor_size(&config.monitor)
        .ok_or(format!("Failed to get monitor size: {:?}", config.monitor))?;

    if let NumOrRelative::Relative(_) = &mut btn_config.extra_trigger_size {
        let max = match config.edge {
            Edge::Left | Edge::Right => monitor_size.0,
            Edge::Top | Edge::Bottom => monitor_size.1,
            _ => unreachable!(),
        };
        btn_config.extra_trigger_size.calculate_relative(max as f64);
    };

    btn_config
        .size
        .ensure_no_relative(monitor_size, config.edge)
}
