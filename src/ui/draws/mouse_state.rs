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
}

impl MouseState {
    fn new() -> Self {
        Self {
            hovering: false,
            pressing: None,
            mouse_debug: crate::args::get_args().mouse_debug,
            cb: None,
        }
    }
    fn press(&mut self, p: u32, pos: (f64, f64)) {
        if self.mouse_debug {
            crate::notify_send(
                "Way-edges mouse button debug message",
                &format!("key released: {}", p),
                false,
            );
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
            crate::notify_send(
                "Way-edges mouse button debug message",
                &format!("key released: {}", p),
                false,
            );
        };

        if self.pressing.eq(&Some(p)) {
            self.pressing = None;
            if let Some(f) = &mut self.cb {
                f(MouseEvent::Release(pos, p))
            }
        }
    }
    fn hover_enter(&mut self, pos: (f64, f64)) {
        self.hovering = true;
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

pub type MouseStateRc = Rc<RefCell<MouseState>>;

pub fn new_mouse_state(darea: &DrawingArea) -> MouseStateRc {
    let ms = Rc::new(RefCell::new(MouseState::new()));
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

pub fn new_translate_mouse_state(
    ts: TransitionStateRc,
    ms: MouseStateRc,
    mut additional_cbs: Option<MouseEventFunc>,
    hidden_only: bool,
) -> (MouseEventFunc, TranslateStateRc) {
    let tls = Rc::new(RefCell::new(TranslateState::new(
        ts.downgrade(),
        ms.downgrade(),
    )));
    (
        new_mouse_event_func(glib::clone!(
            #[strong(rename_to=tls)]
            tls,
            move |e| {
                match e {
                    MouseEvent::Enter(_) | MouseEvent::Leave => {
                        tls.borrow().ensure_direction();
                    }
                    MouseEvent::Release(_, k) => {
                        if !hidden_only && k == BUTTON_MIDDLE {
                            tls.borrow_mut().toggle_pin();
                        }
                        tls.borrow().ensure_direction();
                    }
                    _ => {}
                };
                if let Some(f) = additional_cbs.as_mut() {
                    f(e)
                }
            }
        )),
        tls,
    )
}

pub type TranslateStateRc = Rc<RefCell<TranslateState>>;

pub struct TranslateState {
    is_pinned: bool,
    pop_state: Option<Rc<Cell<bool>>>,
    timeout: Duration,

    ts: Weak<RefCell<TransitionState>>,
    ms: Weak<RefCell<MouseState>>,
}

impl TranslateState {
    pub fn new(ts: Weak<RefCell<TransitionState>>, ms: Weak<RefCell<MouseState>>) -> Self {
        Self {
            pop_state: None,
            timeout: Duration::from_secs(2),
            is_pinned: false,
            ts,
            ms,
        }
    }

    // pin
    pub fn pin(&mut self) {
        self.invalidate_pop();
        self.is_pinned = true;
    }
    pub fn unpin(&mut self) {
        self.is_pinned = false;
    }

    pub fn toggle_pin(&mut self) {
        if self.is_pinned {
            self.unpin();
        } else {
            self.pin();
        }

        if let Some(ts) = self.ts.upgrade() {
            ts.borrow_mut().set_direction_self(self.is_pinned.into());
        }
    }

    pub fn ensure_direction(&self) {
        if let (Some(ts), Some(ms)) = (self.ts.upgrade(), self.ms.upgrade()) {
            // if not pin
            if !self.is_pinned {
                let direction = {
                    let ms_ref = unsafe { ms.as_ptr().as_ref().unwrap() };
                    check_translate_direction(&ms_ref.hovering, &ms_ref.pressing)
                };
                ts.borrow_mut().set_direction_self(direction);
            }
        }
    }

    // pop
    pub fn pop(&mut self, on_end_cb: Option<impl FnOnce() + 'static>) {
        self.invalidate_pop();

        let state = Rc::new(Cell::new(true));
        let state_clone = state.clone();
        self.pop_state = Some(state);
        {
            glib::timeout_add_local_once(self.timeout, move || {
                if state_clone.get() {
                    if let Some(f) = on_end_cb {
                        f()
                    }
                }
            });
        };
    }
    pub fn invalidate_pop(&mut self) {
        if let Some(before) = self.pop_state.take() {
            before.set(false);
        }
    }
}

// NOTE: THIS ONE IS ONLY FOR TRANSLATE_STATE
pub struct TranslateStateExpose {
    pub tls: Weak<RefCell<TranslateState>>,
    pub draw: Box<dyn FnMut()>,
}
impl TranslateStateExpose {
    pub fn new(tls: Weak<RefCell<TranslateState>>, f: impl FnMut() + 'static) -> Self {
        Self {
            tls,
            draw: Box::new(f),
        }
    }
}
impl WidgetExpose for TranslateStateExpose {
    fn toggle_pin(&mut self) {
        if let Some(tls) = self.tls.upgrade() {
            tls.borrow_mut().toggle_pin();
            (self.draw)()
        }
    }
}

/// return is transition change
// pub fn ensure_transition_direction(ms: &MouseStateRc, tls: &TranslateStateRc) {
//     // if not pin
//     if !tls.borrow().is_pinned {
//         let direction = {
//             let ms_ref = unsafe { ms.as_ptr().as_ref().unwrap() };
//             check_translate_direction(&ms_ref.hovering, &ms_ref.pressing)
//         };
//         ts.borrow_mut().set_direction_self(direction);
//     }
// }

pub fn check_translate_direction(hovering: &bool, pressing: &Option<u32>) -> TransitionDirection {
    // not hovering and no pressing
    if !*hovering && pressing.is_none() {
        TransitionDirection::Backward
    } else {
        TransitionDirection::Forward
    }
}
