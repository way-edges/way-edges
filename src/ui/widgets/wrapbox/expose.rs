use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use gtk::{glib, prelude::WidgetExt, DrawingArea};

use crate::ui::{draws::mouse_state::MouseState, WidgetExpose};

pub type BoxRedrawFunc = Rc<dyn Fn()>;

#[derive(Clone)]
pub struct BoxExpose {
    pub redraw_func: BoxRedrawFunc,
}

impl BoxExpose {
    pub fn new(darea: &DrawingArea) -> Self {
        let redraw_func = Rc::new(glib::clone!(
            #[weak]
            darea,
            move || {
                darea.queue_draw();
            }
        ));

        Self { redraw_func }
    }
    pub fn update_func(&self) -> BoxRedrawFunc {
        self.redraw_func.clone()
    }
}

pub struct BoxWidgetExpose {
    ms: Weak<RefCell<MouseState>>,
}
impl BoxWidgetExpose {
    pub fn new(ms: Weak<RefCell<MouseState>>) -> Self {
        Self { ms }
    }
}
impl WidgetExpose for BoxWidgetExpose {
    fn toggle_pin(&mut self) {
        if let Some(ms) = self.ms.upgrade() {
            ms.borrow_mut().toggle_pin();
        }
    }
}
