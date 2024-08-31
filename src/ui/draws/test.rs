use gio::glib::clone::Downgrade;
use gtk::{
    gdk::BUTTON_MIDDLE,
    glib,
    prelude::{GestureSingleExt, WidgetExt},
    DrawingArea, EventControllerMotion, GestureClick,
};
use std::{
    cell::{Cell, RefCell},
    ops::Not,
    rc::{Rc, Weak},
    time::Duration,
};

use crate::ui::WidgetExpose;

use super::transition_state::{TransitionDirection, TransitionState, TransitionStateRc};

#[derive(Debug, Clone)]
pub enum MouseEvent {
    Press((f64, f64), u32),
    Release((f64, f64), u32),
    Enter((f64, f64)),
    Leave,
    Motion((f64, f64)),

    Pin,
    Unpin,
    Pop,
    Unpop,
}

pub type MouseEventFunc = Box<dyn FnMut(MouseEvent) + 'static>;
pub fn new_mouse_event_func(f: impl FnMut(MouseEvent) + 'static) -> MouseEventFunc {
    Box::new(f)
}

pub struct MouseState {
    pub hovering: bool,
    pub pressing: Option<u32>,
    pub mouse_debug: bool,
    pub cb: Option<MouseEventFunc>,

    pub pin_state: bool,
    pub pop_state: Option<Rc<dyn Fn()>>,

    pub ts: TransitionStateRc,

    enable_pin: bool,
    enable_pop: bool,
    enable_hover: bool,
}

impl MouseState {
    fn new(enable_pin: bool, enable_pop: bool, enable_hover: bool, ts: TransitionStateRc) -> Self {
        Self {
            hovering: false,
            pressing: None,
            mouse_debug: crate::args::get_args().mouse_debug,
            cb: None,

            pin_state: false,
            pop_state: None,

            ts,

            enable_pin,
            enable_pop,
            enable_hover,
        }
    }

    // =========== PIN ==============
    pub fn set_pin(&mut self, pin: bool) {
        if !self.enable_pin {
            return;
        }
        if self.pin_state != pin {
            self.pin_state = pin;

            let ts = self.ts.borrow_mut().set_direction_self(pin.into());

            if let Some(f) = &mut self.cb {
                f(match pin {
                    true => MouseEvent::Pin,
                    false => MouseEvent::Unpin,
                })
            }
        }
    }
    pub fn toggle_pin(&mut self) {
        if !self.enable_pin {
            return;
        }
        self.set_pin(!self.pin_state)
    }
    // =========== PIN ==============

    // =========== POP ==============
    pub fn pop(&self) {}

    // triggers
    fn press(&mut self, p: u32, pos: (f64, f64)) {
        if self.mouse_debug {
            let msg = format!("key pressed: {}", p);
            log::debug!("Mouse Debug info: {msg}");
            crate::notify_send("Way-edges mouse button debug message", &msg, false);
        };
        if self.pressing.is_none() {
            self.pressing = Some(p);
            if let Some(f) = &mut self.cb {
                f(MouseEvent::Press(pos, p))
            }
        }
    }
    fn unpress(&mut self, p: u32, pos: (f64, f64)) {
        if self.mouse_debug {
            let msg = format!("key released: {}", p);
            log::debug!("Mouse Debug info: {msg}");
            crate::notify_send("Way-edges mouse button debug message", &msg, false);
        };

        if !self.pin_state && !self.hovering {
            self.ts
                .borrow_mut()
                .set_direction_self(TransitionDirection::Backward);
        }

        if self.pressing.eq(&Some(p)) {
            self.pressing = None;
            if let Some(f) = &mut self.cb {
                f(MouseEvent::Release(pos, p))
            }
        }
    }
    fn hover_enter(&mut self, pos: (f64, f64)) {
        self.hovering = true;

        if !self.pin_state {
            self.ts
                .borrow_mut()
                .set_direction_self(TransitionDirection::Forward);
        }

        if let Some(f) = &mut self.cb {
            f(MouseEvent::Enter(pos))
        }
    }
    fn hover_motion(&mut self, pos: (f64, f64)) {
        if let Some(f) = &mut self.cb {
            f(MouseEvent::Motion(pos))
        }
    }
    fn hover_leave(&mut self) {
        self.hovering = false;

        if !self.pin_state {
            self.ts
                .borrow_mut()
                .set_direction_self(TransitionDirection::Forward);
        }

        if let Some(f) = &mut self.cb {
            f(MouseEvent::Leave)
        }
    }

    pub fn set_event_cb(&mut self, cb: MouseEventFunc) {
        self.cb.replace(cb);
    }
    pub fn take_event_cb(&mut self) -> Option<MouseEventFunc> {
        self.cb.take()
    }
}
impl Drop for MouseState {
    fn drop(&mut self) {
        log::debug!("drop mouse state");
    }
}
