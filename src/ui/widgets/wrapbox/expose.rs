use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use gio::glib::{clone::Downgrade, WeakRef};
use gtk::{glib, prelude::WidgetExt, DrawingArea};

use crate::ui::{draws::mouse_state::MouseState, WidgetExpose};

pub type BoxExposeRc = Rc<RefCell<BoxExpose>>;

pub struct BoxExpose {
    pub darea: WeakRef<DrawingArea>,
}

impl BoxExpose {
    pub fn new(darea: &DrawingArea) -> BoxExposeRc {
        Rc::new(RefCell::new(BoxExpose {
            darea: darea.downgrade(),
        }))
    }
    pub fn update_func(&self) -> impl Fn() + Clone {
        let darea = self.darea.upgrade().expect("DrawingArea should be alive");
        glib::clone!(
            #[weak]
            darea,
            move || {
                darea.queue_draw();
            }
        )
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
