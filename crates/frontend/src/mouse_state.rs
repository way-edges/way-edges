use educe::Educe;
use gio::glib::clone::Downgrade;
use gtk::{
    gdk::BUTTON_MIDDLE,
    glib,
    prelude::{GestureSingleExt, WidgetExt},
    DrawingArea, EventControllerMotion, GestureClick,
};
use std::{rc::Rc, time::Duration};

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

pub type MouseEventFunc = Box<dyn FnMut(&MouseStateData, MouseEvent) + 'static>;

#[derive(Debug)]
pub struct MouseStateData {
    pub hovering: bool,
    pub pressing: Option<u32>,

    pub pin_state: bool,
    pub pop_state: Option<Rc<()>>,
}
impl MouseStateData {
    pub fn new() -> Self {
        Self {
            hovering: false,
            pressing: None,
            pin_state: false,
            pop_state: None,
        }
    }
}

use util::wrap_rc;
wrap_rc!(pub MouseStateRc, pub MouseState);

#[derive(Educe)]
#[educe(Debug)]
pub struct MouseState {
    data: MouseStateData,

    pin_key: u32,
    mouse_debug: bool,
    #[educe(Debug(ignore))]
    cb: Option<MouseEventFunc>,
}

impl MouseState {
    pub fn new() -> Self {
        Self {
            data: MouseStateData::new(),

            mouse_debug: false,
            cb: None,
            pin_key: BUTTON_MIDDLE,
        }
    }

    fn call_event(&mut self, e: MouseEvent) {
        if let Some(f) = &mut self.cb {
            f(&self.data, e)
        }
    }

    pub fn set_event_cb(&mut self, cb: MouseEventFunc) {
        self.cb.replace(cb);
    }

    // =========== PIN ==============
    pub fn set_pin(&mut self, pin: bool) {
        if self.data.pin_state != pin {
            self.data.pin_state = pin;

            self.call_event(match pin {
                true => MouseEvent::Pin,
                false => MouseEvent::Unpin,
            });
        }
    }
    pub fn toggle_pin(&mut self) {
        self.set_pin(!self.data.pin_state)
    }
    // =========== PIN ==============

    // =========== POP ==============
    pub fn pop(&mut self) {
        let sl = self as *mut Self;
        let handle = Rc::new(());
        let handle_weak = handle.downgrade();
        self.data.pop_state = Some(handle);
        let cb = move || {
            if handle_weak.upgrade().is_none() {
                return;
            }
            if let Some(ms) = unsafe { sl.as_mut() } {
                ms.call_event(MouseEvent::Unpop);
            }
        };

        glib::timeout_add_local_once(Duration::from_secs(1), cb);

        self.call_event(MouseEvent::Pop);
    }
    fn invalidate_pop(&mut self) {
        self.data.pop_state.take();
    }
    // =========== POP ==============

    // triggers
    fn press(&mut self, p: u32, pos: (f64, f64)) {
        if self.mouse_debug {
            let msg = format!("key pressed: {}", p);
            log::debug!("Mouse Debug info: {msg}");
            util::notify_send("Way-edges mouse button debug message", &msg, false);
        };

        if self.data.pressing.is_none() {
            self.data.pressing = Some(p);
            self.call_event(MouseEvent::Press(pos, p));
        }
    }
    fn unpress(&mut self, p: u32, pos: (f64, f64)) {
        if self.mouse_debug {
            let msg = format!("key released: {}", p);
            log::debug!("Mouse Debug info: {msg}");
            util::notify_send("Way-edges mouse button debug message", &msg, false);
        };

        if p == self.pin_key {
            self.toggle_pin();
        }

        if self.data.pressing.eq(&Some(p)) {
            self.data.pressing = None;
            self.call_event(MouseEvent::Release(pos, p));
        }
    }
    fn hover_enter(&mut self, pos: (f64, f64)) {
        self.data.hovering = true;
        self.call_event(MouseEvent::Enter(pos));
    }
    fn hover_motion(&mut self, pos: (f64, f64)) {
        self.call_event(MouseEvent::Motion(pos));
    }
    fn hover_leave(&mut self) {
        self.data.hovering = false;
        self.call_event(MouseEvent::Leave);
    }
}
impl MouseState {
    fn connect(self, darea: &DrawingArea) -> MouseStateRc {
        let ms = self.make_rc();
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
}
impl Drop for MouseState {
    fn drop(&mut self) {
        log::debug!("drop mouse state");
    }
}
