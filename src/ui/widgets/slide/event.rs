use std::cell::Cell;
use std::time::{Duration, Instant};
use std::{cell::RefCell, rc::Rc};

use gtk::gdk::{BUTTON_MIDDLE, BUTTON_PRIMARY};
use gtk::prelude::{GestureSingleExt, WidgetExt};
use gtk::{glib, EventControllerMotion};
use gtk::{DrawingArea, GestureClick};
use gtk4_layer_shell::Edge;

use crate::ui::draws::util::Z;
use crate::ui::draws::{mouse_state::BaseMouseState, transition_state::TransitionState};

#[derive(Clone, Copy)]
pub enum Direction {
    Forward,
    Backward,
}

#[derive(Clone)]
pub struct ProgressState {
    pub max: f64,
    /// 0 ~ 1
    pub current: Rc<Cell<f64>>,
    pub direction: Direction,
}
impl ProgressState {
    pub fn new(max: f64, direction: Direction) -> Self {
        Self {
            max,
            current: Rc::new(Cell::new(0.0)),
            direction,
        }
    }
    pub fn set_progress_raw(&self, c: f64) {
        self.current.set(c);
    }
    pub fn set_progress(&self, mut c: f64) {
        if c < Z {
            c = Z;
        } else if c > self.max {
            c = self.max;
        }
        let c = match self.direction {
            Direction::Forward => c,
            Direction::Backward => self.max - c,
        };
        self.set_progress_raw(c / self.max);
    }
}

#[derive(Clone, Copy)]
pub enum XorY {
    X,
    Y,
}
impl From<Edge> for XorY {
    fn from(e: Edge) -> Self {
        match e {
            Edge::Left | Edge::Right => Self::Y,
            Edge::Top | Edge::Bottom => Self::X,
            _ => unreachable!(),
        }
    }
}

pub(super) fn setup_event(
    darea: &DrawingArea,
    ts: &TransitionState<f64>,
    xory: XorY,
    direction: Direction,
    max: f64,
) -> Rc<Cell<f64>> {
    let mouse_state = Rc::new(RefCell::new(BaseMouseState::new(ts)));
    let progress_state = Rc::new(RefCell::new(ProgressState::new(max, direction)));
    set_event_mouse_click(darea, mouse_state.clone(), progress_state.clone(), xory);
    set_event_mouse_move(darea, mouse_state.clone(), progress_state.clone(), xory);
    let progress = progress_state.borrow().current.clone();
    progress
}

fn set_event_mouse_click(
    darea: &DrawingArea,
    mouse_state: Rc<RefCell<BaseMouseState>>,
    progress_state: Rc<RefCell<ProgressState>>,
    xory: XorY,
) {
    let show_mouse_debug = crate::args::get_args().mouse_debug;
    let click_control = GestureClick::builder().button(0).exclusive(true).build();
    let click_done_cb = move |mouse_state: &Rc<RefCell<BaseMouseState>>, darea: &DrawingArea| {
        if let Some(btn) = mouse_state.borrow_mut().set_pressing(None) {
            if show_mouse_debug {
                crate::notify_send(
                    "Way-edges mouse button debug message",
                    &format!("key released: {}", btn),
                    false,
                );
            };
            darea.queue_draw();
        } else {
            log::debug!("No pressing button in mouse_state");
        }
    };

    click_control.connect_pressed(
        glib::clone!(@strong mouse_state, @strong progress_state, @weak darea => move |g, _, x, y| {
            let btn = g.current_button();
            if show_mouse_debug {
                crate::notify_send("Way-edges mouse button debug message", &format!("key pressed: {}", btn), false);
            };
            // middle clike to pin
            if btn == BUTTON_MIDDLE {
                mouse_state.borrow().toggle_pin();
            }
            mouse_state.borrow_mut().set_pressing(Some(btn));
            if btn == BUTTON_PRIMARY {
                let progress = match xory {
                    XorY::X => x,
                    XorY::Y => y,
                };
                progress_state.borrow().set_progress(progress);
            }
            darea.queue_draw();
        }),
    );
    click_control.connect_released(
        glib::clone!(@strong mouse_state, @weak darea => move |_, _, _, _| {
            click_done_cb(&mouse_state, &darea);
        }),
    );
    click_control.connect_unpaired_release(
        glib::clone!(@strong mouse_state, @weak darea => move |_, _, _, d, _| {
            if mouse_state.borrow().pressing.get() == Some(d) {
                click_done_cb(&mouse_state, &darea);
            }
        }),
    );
    darea.add_controller(click_control);
}

fn set_event_mouse_move(
    darea: &DrawingArea,
    mouse_state: Rc<RefCell<BaseMouseState>>,
    progress_state: Rc<RefCell<ProgressState>>,
    xory: XorY,
) {
    let motion = EventControllerMotion::new();
    motion.connect_enter(
        glib::clone!(@strong mouse_state, @weak darea => move |_, _, _| {
            log::debug!("Mouse enter slide widget");
            mouse_state.borrow_mut().set_hovering(true);
            darea.queue_draw();
        }),
    );
    motion.connect_leave(glib::clone!(@strong mouse_state, @weak darea=> move |_,| {
        log::debug!("Mouse leave slide widget");
        mouse_state.borrow_mut().set_hovering(false);
        darea.queue_draw();
    }));
    motion.connect_motion(
        glib::clone!(@strong mouse_state, @strong progress_state, @weak darea => move |_, x, y| {
            if mouse_state.borrow().pressing.get() == Some(BUTTON_PRIMARY) {
                let progress = match xory {
                    XorY::X => x,
                    XorY::Y => y,
                };
                log::debug!("Change progress: {progress}");
                progress_state.borrow().set_progress(progress);
                darea.queue_draw();
            }
        }),
    );
    darea.add_controller(motion);
}
