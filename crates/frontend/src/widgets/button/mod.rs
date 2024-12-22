mod draw;
mod event;

use std::cell::Cell;
use std::rc::Rc;

use crate::window::WindowContext;
use config::{widgets::button::BtnConfig, Config};

pub fn init_widget(window: &mut WindowContext, config: Config, mut btn_config: BtnConfig) {
    let pressing_state = Rc::new(Cell::new(false));
    draw::setup_draw(window, &btn_config, config.edge, pressing_state.clone());
    event::setup_event(window, pressing_state, &mut btn_config);
}
