use gtk::{
    gdk::BUTTON_MIDDLE,
    glib,
    prelude::{GestureSingleExt, WidgetExt},
    DrawingArea, EventControllerMotion, GestureClick,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use crate::ui::widgets::wrapbox::MousePosition;

use super::transition_state::{TransitionDirection, TransitionStateRc};

pub struct MouseStateCbs {
    pub hover_enter_cb: Option<Box<dyn FnMut(MousePosition)>>,
    pub hover_leave_cb: Option<Box<dyn FnMut()>>,
    pub hover_motion_cb: Option<Box<dyn FnMut(MousePosition)>>,
    pub press_cb: Option<Box<dyn FnMut(MousePosition, u32)>>,
    pub unpress_cb: Option<Box<dyn FnMut(MousePosition, u32)>>,
}
impl MouseStateCbs {
    pub fn new() -> Self {
        Self {
            hover_enter_cb: None,
            hover_leave_cb: None,
            hover_motion_cb: None,
            press_cb: None,
            unpress_cb: None,
        }
    }
    pub fn set_hover_enter_cb<F>(&mut self, f: F)
    where
        F: FnMut(MousePosition) + 'static,
    {
        self.hover_enter_cb = Some(Box::new(f))
    }

    pub fn set_hover_leave_cb<F>(&mut self, f: F)
    where
        F: FnMut() + 'static,
    {
        self.hover_leave_cb = Some(Box::new(f))
    }

    pub fn set_hover_motion_cb<F>(&mut self, f: F)
    where
        F: FnMut(MousePosition) + 'static,
    {
        self.hover_motion_cb = Some(Box::new(f))
    }

    pub fn set_press_cb<F>(&mut self, f: F)
    where
        F: FnMut(MousePosition, u32) + 'static,
    {
        self.press_cb = Some(Box::new(f))
    }

    pub fn set_unpress_cb<F>(&mut self, f: F)
    where
        F: FnMut(MousePosition, u32) + 'static,
    {
        self.unpress_cb = Some(Box::new(f))
    }
}

pub struct MouseState {
    pub hovering: bool,
    pub pressing: Option<u32>,
    pub mouse_debug: bool,
    pub cbs: MouseStateCbs,
}

impl MouseState {
    fn new() -> Self {
        Self {
            hovering: false,
            pressing: None,
            cbs: MouseStateCbs::new(),
            mouse_debug: crate::args::get_args().mouse_debug,
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
            if let Some(f) = &mut self.cbs.press_cb {
                f(pos, p)
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
            if let Some(f) = &mut self.cbs.unpress_cb {
                f(pos, p)
            }
        }
    }
    fn hover_enter(&mut self, pos: (f64, f64)) {
        self.hovering = true;
        if let Some(f) = &mut self.cbs.hover_enter_cb {
            f(pos)
        }
    }
    fn hover_motion(&mut self, pos: (f64, f64)) {
        if let Some(f) = &mut self.cbs.hover_motion_cb {
            f(pos)
        }
    }
    fn hover_leave(&mut self) {
        self.hovering = false;
        if let Some(f) = &mut self.cbs.hover_leave_cb {
            f()
        }
    }

    pub fn set_cbs(&mut self, cbs: MouseStateCbs) {
        self.cbs = cbs
    }
    pub fn cbs(&mut self) -> &mut MouseStateCbs {
        &mut self.cbs
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
    mut additional_cbs: Option<MouseStateCbs>,
    hidden_only: bool,
) -> MouseStateCbs {
    let tls = Rc::new(RefCell::new(TranslateState::new()));
    let mut cbs = MouseStateCbs::new();
    // enter
    {
        let mut f = additional_cbs
            .as_mut()
            .and_then(|f| f.hover_enter_cb.take());
        cbs.set_hover_enter_cb(glib::clone!(
            #[strong(rename_to=tls)]
            tls,
            #[weak(rename_to=ts)]
            ts,
            #[weak(rename_to=ms)]
            ms,
            move |pos| {
                let tls = if hidden_only { None } else { Some(&tls) };
                ensure_transition_direction(&ts, &ms, tls);
                if let Some(f) = f.as_mut() {
                    f(pos)
                }
            }
        ));
    }
    // leave
    {
        let mut f = additional_cbs
            .as_mut()
            .and_then(|f| f.hover_leave_cb.take());
        cbs.set_hover_leave_cb(glib::clone!(
            #[strong(rename_to=tls)]
            tls,
            #[weak(rename_to=ts)]
            ts,
            #[weak(rename_to=ms)]
            ms,
            move || {
                let tls = if hidden_only { None } else { Some(&tls) };
                ensure_transition_direction(&ts, &ms, tls);
                if let Some(f) = f.as_mut() {
                    f()
                }
            }
        ));
    }
    // release
    {
        let mut f = additional_cbs.as_mut().and_then(|f| f.unpress_cb.take());
        cbs.set_unpress_cb(glib::clone!(
            #[strong(rename_to=tls)]
            tls,
            #[weak(rename_to=ts)]
            ts,
            #[weak(rename_to=ms)]
            ms,
            move |pos, k| {
                if !hidden_only && k == BUTTON_MIDDLE {
                    tls.borrow_mut().toggle_pin();
                }
                ensure_transition_direction(&ts, &ms, Some(&tls));
                if let Some(f) = f.as_mut() {
                    f(pos, k)
                }
            }
        ));
    }
    cbs
}

pub type TranslateStateRc = Rc<RefCell<TranslateState>>;

pub struct TranslateState {
    is_pinned: bool,
    pop_state: Option<Rc<Cell<bool>>>,
    timeout: Duration,
}

impl TranslateState {
    pub fn new() -> Self {
        Self {
            pop_state: None,
            timeout: Duration::from_secs(2),
            is_pinned: false,
        }
    }

    // pin
    pub fn pin(&mut self) {
        println!("hr");
        self.invalidate_pop();
        self.is_pinned = true;
    }
    pub fn unpin(&mut self) {
        self.is_pinned = false;
    }
    /// return the pin state
    pub fn toggle_pin(&mut self) -> bool {
        if self.is_pinned {
            self.unpin();
            false
        } else {
            self.pin();
            true
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

/// return is transition change
pub fn ensure_transition_direction(
    ts: &TransitionStateRc,
    ms: &MouseStateRc,
    tls: Option<&TranslateStateRc>,
) {
    if tls.is_none() || !tls.unwrap().borrow().is_pinned {
        let direction = {
            let ms_ref = unsafe { ms.as_ptr().as_ref().unwrap() };
            check_translate_direction(&ms_ref.hovering, &ms_ref.pressing)
        };
        ts.borrow_mut().set_direction_self(direction);
    }
}

pub fn check_translate_direction(hovering: &bool, pressing: &Option<u32>) -> TransitionDirection {
    // not hovering and no pressing
    if !*hovering && pressing.is_none() {
        TransitionDirection::Backward
    } else {
        TransitionDirection::Forward
    }
}
