use gio::glib::clone::Downgrade;
use gtk::{
    gdk::BUTTON_MIDDLE,
    glib,
    prelude::{GestureSingleExt, WidgetExt},
    DrawingArea, EventControllerMotion, GestureClick,
};
use std::{cell::RefCell, rc::Rc, time::Duration};

use super::transition_state::{TransitionDirection, TransitionStateRc};

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
    pub fn new(
        enable_pin: bool,
        enable_pop: bool,
        enable_hover: bool,
        ts: TransitionStateRc,
    ) -> Self {
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

            if pin {
                self.set_ts_dir(TransitionDirection::Forward);
            } else if !self.hovering && self.pressing.is_none() && self.pop_state.is_none() {
                self.set_ts_dir(TransitionDirection::Backward);
            }

            self.call_event(match pin {
                true => MouseEvent::Pin,
                false => MouseEvent::Unpin,
            });
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
    pub fn pop(&mut self) {
        if !self.enable_pop {
            return;
        }

        self.set_ts_dir(TransitionDirection::Forward);

        let sl = self as *mut Self;
        let cb = {
            let cb = Rc::new(move || {
                if let Some(ms) = unsafe { sl.as_mut() } {
                    if !ms.hovering && ms.pressing.is_none() && !ms.pin_state {
                        ms.set_ts_dir(TransitionDirection::Backward);
                    }
                    ms.call_event(MouseEvent::Unpop);
                }
            });

            self.pop_state = Some(cb.clone());
            cb.downgrade()
        };

        glib::timeout_add_local_once(Duration::from_secs(1), move || {
            if let Some(f) = cb.upgrade() {
                f()
            }
        });

        self.call_event(MouseEvent::Pop);
    }
    fn invalidate_pop(&mut self) {
        self.pop_state.take();
    }
    // =========== POP ==============

    // triggers
    fn press(&mut self, p: u32, pos: (f64, f64)) {
        if self.mouse_debug {
            let msg = format!("key pressed: {}", p);
            log::debug!("Mouse Debug info: {msg}");
            crate::notify_send("Way-edges mouse button debug message", &msg, false);
        };

        if self.pressing.is_none() {
            self.pressing = Some(p);
            self.call_event(MouseEvent::Press(pos, p));
        }
    }
    fn unpress(&mut self, p: u32, pos: (f64, f64)) {
        if self.mouse_debug {
            let msg = format!("key released: {}", p);
            log::debug!("Mouse Debug info: {msg}");
            crate::notify_send("Way-edges mouse button debug message", &msg, false);
        };

        if self.enable_pin && p == BUTTON_MIDDLE {
            self.toggle_pin();
        }

        if !self.pin_state && !self.hovering {
            self.set_ts_dir(TransitionDirection::Backward);
        }

        if self.pressing.eq(&Some(p)) {
            self.pressing = None;
            self.call_event(MouseEvent::Release(pos, p));
        }
    }
    fn hover_enter(&mut self, pos: (f64, f64)) {
        self.hovering = true;

        if !self.pin_state {
            self.set_ts_dir(TransitionDirection::Forward);
        }

        self.call_event(MouseEvent::Enter(pos));
    }
    fn hover_motion(&mut self, pos: (f64, f64)) {
        self.call_event(MouseEvent::Motion(pos));
    }
    fn hover_leave(&mut self) {
        self.hovering = false;

        if !self.pin_state && self.pressing.is_none() {
            self.set_ts_dir(TransitionDirection::Backward);
        }

        self.call_event(MouseEvent::Leave);
    }

    fn set_ts_dir(&mut self, dir: TransitionDirection) {
        self.invalidate_pop();
        self.ts.borrow_mut().set_direction_self(dir);
    }
    fn call_event(&mut self, e: MouseEvent) {
        if let Some(f) = &mut self.cb {
            f(e)
        }
    }

    pub fn set_event_cb(&mut self, cb: MouseEventFunc) {
        self.cb.replace(cb);
    }
    // pub fn take_event_cb(&mut self) -> Option<MouseEventFunc> {
    //     self.cb.take()
    // }
}
impl Drop for MouseState {
    fn drop(&mut self) {
        log::debug!("drop mouse state");
    }
}

pub type MouseStateRc = Rc<RefCell<MouseState>>;

/// all strong rc
pub fn new_mouse_state(darea: &DrawingArea, ms: MouseState) -> MouseStateRc {
    let ms = Rc::new(RefCell::new(ms));
    {
        let click_control = GestureClick::builder().button(0).exclusive(true).build();
        click_control.connect_pressed(glib::clone!(
            #[strong]
            ms,
            move |g, _, x, y| {
                ms.borrow_mut().press(g.current_button(), (x, y));
            }
        ));
        click_control.connect_released(glib::clone!(
            #[strong]
            ms,
            move |g, _, x, y| {
                ms.borrow_mut().unpress(g.current_button(), (x, y));
            }
        ));
        click_control.connect_unpaired_release(glib::clone!(
            #[strong]
            ms,
            move |_, x, y, d, _| {
                ms.borrow_mut().unpress(d, (x, y));
            }
        ));
        darea.add_controller(click_control);
    };
    {
        let motion = EventControllerMotion::new();
        motion.connect_enter(glib::clone!(
            #[strong]
            ms,
            move |_, x, y| {
                ms.borrow_mut().hover_enter((x, y));
            }
        ));
        motion.connect_leave(glib::clone!(
            #[strong]
            ms,
            move |_| {
                ms.borrow_mut().hover_leave();
            }
        ));
        motion.connect_motion(glib::clone!(
            #[strong]
            ms,
            move |_, x, y| {
                ms.borrow_mut().hover_motion((x, y));
            }
        ));
        darea.add_controller(motion);
    }
    ms
}
