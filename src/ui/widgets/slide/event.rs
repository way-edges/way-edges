use std::cell::Cell;
use std::{cell::RefCell, rc::Rc};

use gtk::gdk::{BUTTON_MIDDLE, BUTTON_PRIMARY};
use gtk::prelude::{GestureSingleExt, WidgetExt};
use gtk::{glib, EventControllerMotion};
use gtk::{DrawingArea, GestureClick};
use gtk4_layer_shell::Edge;

use crate::config::widgets::common::EventMap;
use crate::config::widgets::slide::{Direction, Task};
use crate::ui::draws::util::Z;
use crate::ui::draws::{mouse_state::BaseMouseState, transition_state::TransitionState};

pub struct ProgressState {
    pub max: f64,
    /// 0 ~ 1
    pub current: Rc<Cell<f64>>,
    pub direction: Direction,
    pub on_change: Option<Task>,
}
impl ProgressState {
    pub fn new(max: f64, direction: Direction, on_change: Option<Task>) -> Self {
        Self {
            max,
            current: Rc::new(Cell::new(0.0)),
            direction,
            on_change,
        }
    }
    pub fn set_progress_raw(&mut self, c: f64) -> bool {
        if let Some(ref mut f) = &mut self.on_change {
            if !f(c) {
                return false;
            };
        };
        self.current.set(c);
        true
    }
    pub fn set_progress(&mut self, mut c: f64) -> bool {
        if c < Z {
            c = Z;
        } else if c > self.max {
            c = self.max;
        }
        let c = match self.direction {
            Direction::Forward => c,
            Direction::Backward => self.max - c,
        };
        self.set_progress_raw(c / self.max)
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
    on_change: Option<Task>,
    event_map: EventMap,
) -> Rc<Cell<f64>> {
    let mouse_state = Rc::new(RefCell::new(BaseMouseState::new(ts)));
    let progress_state = Rc::new(RefCell::new(ProgressState::new(max, direction, on_change)));
    set_event_mouse_click(
        darea,
        mouse_state.clone(),
        progress_state.clone(),
        xory,
        event_map,
    );
    set_event_mouse_move(darea, mouse_state.clone(), progress_state.clone(), xory);
    let progress = progress_state.borrow().current.clone();
    progress
}

fn set_event_mouse_click(
    darea: &DrawingArea,
    mouse_state: Rc<RefCell<BaseMouseState>>,
    progress_state: Rc<RefCell<ProgressState>>,
    xory: XorY,
    event_map: EventMap,
) {
    let show_mouse_debug = crate::args::get_args().mouse_debug;
    let click_control = GestureClick::builder().button(0).exclusive(true).build();

    let click_done_cb = {
        let cbs = Rc::new(RefCell::new(event_map));
        let mouse_state = mouse_state.clone();
        move |darea: &DrawingArea| {
            if let Some(btn) = mouse_state.borrow_mut().set_pressing(None) {
                if show_mouse_debug {
                    crate::notify_send(
                        "Way-edges mouse button debug message",
                        &format!("key released: {}", btn),
                        false,
                    );
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
                if !progress_state.borrow_mut().set_progress(progress) {
                    return;
                }
            }
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
                if progress_state.borrow_mut().set_progress(progress) {
                    darea.queue_draw();
                };
            }
        }),
    );
    darea.add_controller(motion);
}
