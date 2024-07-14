use crate::config::widgets::common::EventMap;
use crate::ui::draws::transition_state::TransitionState;

use gtk::glib;
use gtk::prelude::*;
use gtk::EventControllerMotion;
use gtk::{DrawingArea, GestureClick};
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

pub(super) fn setup_event(
    darea: &DrawingArea,
    event_map: EventMap,
    ts: &TransitionState<f64>,
) -> Rc<RefCell<MouseState>> {
    let mouse_state = Rc::new(RefCell::new(MouseState::new(ts)));
    set_event_mouse_click(darea, event_map, mouse_state.clone());
    set_event_mouse_move(darea, mouse_state.clone());
    mouse_state
}

pub(super) struct MouseState {
    pub(super) hovering: bool,
    pub(super) pressing: Rc<Cell<Option<u32>>>,

    // transition_state related
    pub(super) t: Rc<Cell<Instant>>,
    pub(super) is_forward: Rc<Cell<bool>>,
    pub(super) max_time: Duration,
}
impl MouseState {
    pub(super) fn new(ts: &TransitionState<f64>) -> Self {
        Self {
            hovering: false,
            pressing: Rc::new(Cell::new(None)),
            t: ts.t.clone(),
            is_forward: ts.is_forward.clone(),
            max_time: ts.duration.get(),
        }
    }
    fn set_transition(&self, open: bool) {
        TransitionState::<f64>::set_direction(&self.t, self.max_time, &self.is_forward, open);
    }
    pub(super) fn set_hovering(&mut self, h: bool) {
        self.hovering = h;
        if !h && self.pressing.get().is_none() {
            self.set_transition(false);
        } else {
            self.set_transition(true);
        }
    }
    pub(super) fn set_pressing(&mut self, p: u32) {
        self.pressing.set(Some(p));
    }
    pub(super) fn take_pressing(&mut self) -> Option<u32> {
        if let Some(old) = self.pressing.take() {
            if !self.hovering {
                self.set_transition(false);
            };
            Some(old)
        } else {
            None
        }
    }
}

fn set_event_mouse_click(
    darea: &DrawingArea,
    event_map: EventMap,
    mouse_state: Rc<RefCell<MouseState>>,
) {
    let show_mouse_debug = crate::args::get_args().mouse_debug;
    let click_control = GestureClick::builder().button(0).exclusive(true).build();

    let click_done_cb = {
        let cbs = Rc::new(RefCell::new(event_map));
        let mouse_state = mouse_state.clone();
        move |darea: &DrawingArea| {
            if let Some(btn) = mouse_state.borrow_mut().take_pressing() {
                if show_mouse_debug {
                    notify(&format!("key released: {}", btn));
                };
                if let Some(cb) = cbs.borrow_mut().get_mut(&btn) {
                    cb();
                };
                darea.queue_draw();
            } else {
                log::debug!("No pressing button in mouse_state");
            }
        }
    };

    click_control.connect_pressed(
        glib::clone!(@strong mouse_state, @weak darea => move |g, _, _, _| {
            let btn = g.current_button();
            if show_mouse_debug {
                notify(&format!("key pressed: {}", btn));
            };
            mouse_state.borrow_mut().set_pressing(btn);
            darea.queue_draw();
        }),
    );
    click_control.connect_released(
        glib::clone!(@strong click_done_cb, @weak darea => move |_, _, _, _| {
            click_done_cb(&darea);
        }),
    );
    click_control.connect_unpaired_release(
        glib::clone!(@strong mouse_state, @weak darea => move |_, _, _, d, _| {
            if mouse_state.borrow().pressing.get() == Some(d) {
                click_done_cb(&darea);
            }
        }),
    );
    darea.add_controller(click_control);
}

fn set_event_mouse_move(darea: &DrawingArea, mouse_state: Rc<RefCell<MouseState>>) {
    let motion = EventControllerMotion::new();
    motion.connect_enter(
        glib::clone!(@strong mouse_state, @weak darea => move |_, _, _| {
            mouse_state.borrow_mut().set_hovering(true);
            darea.queue_draw();
        }),
    );
    motion.connect_leave(glib::clone!(@strong mouse_state, @weak darea=> move |_,| {
        mouse_state.borrow_mut().set_hovering(false);
        darea.queue_draw();
    }));
    darea.add_controller(motion);
}

fn notify(body: &str) {
    let mut n = notify_rust::Notification::new();
    let res = n
        .summary("Way-edges mouse button debug message")
        .body(body)
        .show();
    if let Err(e) = res {
        log::error!("Error sending notification: {}", e);
    }
}
