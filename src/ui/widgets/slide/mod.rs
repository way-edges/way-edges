mod draw;
mod event;
mod pre_draw;

use crate::{
    activate::get_working_area_size,
    config::{Config, NumOrRelative},
};
use gtk::ApplicationWindow;

pub fn init_widget(window: &ApplicationWindow, config: Config) -> Result<gtk::DrawingArea, String> {
    draw::setup_draw(window, config)
}

// pub fn init_widget(
//     window: &ApplicationWindow,
//     config: Config,
//     // mut slide_cfg: SlideConfig,
// ) -> Result<gtk::DrawingArea, String> {
// }