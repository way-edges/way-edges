use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use async_channel::{Receiver, Sender};

use crate::ui::{draws::mouse_state::MouseState, WidgetExpose};

pub type UpdateSignal = ();
pub type BoxExposeRc = Rc<RefCell<BoxExpose>>;

pub struct BoxExpose {
    pub update_signal: Sender<UpdateSignal>,
}

impl BoxExpose {
    pub fn new() -> (BoxExposeRc, Receiver<UpdateSignal>) {
        let (update_signal_sender, update_signal_receiver) = async_channel::bounded(1);
        let se = Rc::new(RefCell::new(BoxExpose {
            update_signal: update_signal_sender,
        }));
        (se, update_signal_receiver)
    }
    pub fn update_func(&self) -> impl Fn() + Clone {
        let s = self.update_signal.downgrade();
        move || {
            if let Some(s) = s.upgrade() {
                // ignored result
                s.force_send(()).ok();
            }
        }
    }
}

pub struct BoxWidgetExpose {
    ms: Weak<RefCell<MouseState>>,
    box_expose: BoxExposeRc,
}
impl BoxWidgetExpose {
    pub fn new(ms: Weak<RefCell<MouseState>>, box_expose: BoxExposeRc) -> Self {
        Self { box_expose, ms }
    }
}
impl WidgetExpose for BoxWidgetExpose {
    fn toggle_pin(&mut self) {
        if let Some(ms) = self.ms.upgrade() {
            ms.borrow_mut().toggle_pin();
        }
    }
    fn close(&mut self) {
        self.box_expose.borrow_mut().update_signal.close();
    }
}
