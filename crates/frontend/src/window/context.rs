use std::{cell::Cell, rc::Rc};

use super::event::WindowPopStateRc;
use config::MonitorSpecifier;
use gtk::{
    prelude::{GtkWindowExt, WidgetExt},
    ApplicationWindow, DrawingArea,
};

use crate::mouse_state::MouseStateRc;

pub struct WindowContext {
    pub name: String,
    pub monitor: MonitorSpecifier,
    pub window: ApplicationWindow,
    pub drawing_area: DrawingArea,

    pub start_pos: Rc<Cell<(i32, i32)>>,
    pub mouse_state: MouseStateRc,
    pub window_pop_state: WindowPopStateRc,
}

impl WindowContext {
    pub fn show(&self) {
        self.window.present();
    }

    pub fn close(&mut self) {
        self.window.close();
        self.window.destroy();
    }
    pub fn toggle_pin(&self) {
        self.window_pop_state
            .borrow_mut()
            .toggle_pin(self.mouse_state.borrow().is_hovering());
        self.drawing_area.queue_draw();
    }
}

impl Drop for WindowContext {
    fn drop(&mut self) {
        self.close()
    }
}
