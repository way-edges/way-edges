use super::draws;
use super::draws::transition_state::TransitionState;
use crate::data;
use gtk::cairo::Context;
use gtk::cairo::LinearGradient;
use gtk::cairo::RectangleInt;
use gtk::cairo::Region;
use gtk::gdk::BUTTON_PRIMARY;
use gtk::gdk::{self, prelude::*, RGBA};
use gtk::glib;
use gtk::prelude::*;
use gtk::EventControllerMotion;
use gtk::{DrawingArea, GestureClick};
use interval_task::runner;
use interval_task::runner::ExternalRunnerExt;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

pub type EventMap = HashMap<u32, Box<dyn Fn()>>;

pub struct MouseState {
    hovering: bool,
    pressing: Rc<Cell<Option<u32>>>,

    // transition_state related
    t: Rc<Cell<Instant>>,
    is_forward: Rc<Cell<bool>>,
    max_time: Duration,
}
impl MouseState {
    pub fn new(ts: &TransitionState<f64>) -> Self {
        Self {
            hovering: false,
            pressing: Rc::new(Cell::new(None)),
            t: ts.t.clone(),
            is_forward: ts.is_forward.clone(),
            max_time: ts.duration,
        }
    }
    fn set_transition(&self, open: bool) {
        TransitionState::<f64>::set_direction(&self.t, self.max_time, &self.is_forward, open);
    }
    pub fn set_hovering(&mut self, h: bool) {
        self.hovering = h;
        if !h && self.pressing.get().is_none() {
            self.set_transition(false);
        } else {
            self.set_transition(true);
        }
    }
    pub fn set_pressing(&mut self, p: u32) {
        self.pressing.set(Some(p));
    }
    pub fn take_pressing(&mut self) -> u32 {
        let old = self.pressing.take().unwrap();
        if !self.hovering {
            self.set_transition(false);
        };
        old
    }
}

struct FrameManager {
    runner: Option<runner::Runner<runner::Task>>,
    frame_gap: Duration,
}
impl FrameManager {
    fn new(frame_rate: u64) -> Self {
        Self {
            runner: None,
            frame_gap: Duration::from_micros(1_000_000 / frame_rate),
        }
    }
    fn start(&mut self, darea: &DrawingArea) {
        if self.runner.is_some() {
            return;
        }
        let (r, mut runner) = interval_task::channel::new(self.frame_gap);
        runner.start().unwrap();
        self.runner = Some(runner);
        glib::spawn_future_local(glib::clone!(@weak darea => async move {
            while r.recv().await.is_ok() {
                darea.queue_draw();
            }
        }));
    }
    fn stop(&mut self) {
        if let Some(runner) = self.runner.take() {
            runner.close().unwrap();
        }
    }
}

pub fn setup_draw(window: &gtk::ApplicationWindow, size: (f64, f64), cbs: EventMap) -> DrawingArea {
    let darea = DrawingArea::new();
    let map_size = ((size.0 as i32 + data::GLOW_SIZE as i32), size.1 as i32);
    darea.set_width_request(map_size.0);
    darea.set_height_request(map_size.1);
    let draw = make_draw_fn(map_size, size);
    let ts = TransitionState::new(Duration::from_millis(100), size.0, 0.);
    let mouse_state = MouseState::new(&ts);
    let is_pressing = mouse_state.pressing.clone();
    let mut frame_manager = FrameManager::new(144);
    darea.set_draw_func(glib::clone!(@weak window =>move |darea, context, _, _| {
        let visible_y = ts.get_y();
        let transition_y = -size.0 + visible_y;
        if transition_y == 0. || transition_y == -size.0 {
            frame_manager.stop();
        }else {
            frame_manager.start(darea);
        }
        context.translate(transition_y, 0.);
        draw(context, is_pressing.get().is_some());
        window.surface().unwrap().set_input_region(
            &Region::create_rectangle(&RectangleInt::new(
                0,
                0,
                visible_y as i32 + data::GLOW_SIZE as i32,
                size.1 as i32,
            ))
        );
    }));
    let mouse_state = Rc::new(RefCell::new(mouse_state));
    set_event_mouse_click(&darea, cbs, mouse_state.clone());
    set_event_mouse_move(&darea, mouse_state);
    window.set_child(Some(&darea));
    darea
}

fn make_draw_fn(map_size: (i32, i32), size: (f64, f64)) -> impl Fn(&Context, bool) {
    let (b, n, p) = draws::pre_draw::draw_to_surface(map_size, size);
    let f_map_size = (map_size.0 as f64, map_size.1 as f64);

    move |ctx: &Context, pressing: bool| {
        // base_surface
        ctx.set_source_surface(&b, 0., 0.).unwrap();
        ctx.rectangle(0., 0., f_map_size.0, f_map_size.1);
        ctx.fill().unwrap();

        // mask
        if pressing {
            ctx.set_source_surface(&p, 0., 0.).unwrap();
        } else {
            ctx.set_source_surface(&n, 0., 0.).unwrap();
        }
        ctx.rectangle(0., 0., f_map_size.0, f_map_size.1);
        ctx.fill().unwrap();
    }
}

fn set_event_mouse_click(
    darea: &DrawingArea,
    event_map: EventMap,
    mouse_state: Rc<RefCell<MouseState>>,
) {
    let click_control = GestureClick::builder().button(0).exclusive(true).build();
    let cbs = Rc::new(event_map);
    let click_done_cb = move |mouse_state: &Rc<RefCell<MouseState>>,
                              darea: &DrawingArea,
                              event_map: &Rc<EventMap>| {
        let key = mouse_state.borrow_mut().take_pressing();
        darea.queue_draw();
        if let Some(cb) = event_map.get(&key) {
            cb()
        };
    };

    click_control.connect_pressed(
        glib::clone!(@strong mouse_state, @weak darea => move |g, _, _, _| {
            println!("key: {}", g.current_button());
            mouse_state.borrow_mut().set_pressing(g.current_button());
            darea.queue_draw();
        }),
    );
    click_control.connect_released(
        glib::clone!(@strong mouse_state, @strong cbs, @weak darea => move |_, _, _, _| {
            click_done_cb(&mouse_state, &darea, &cbs);
        }),
    );
    click_control.connect_unpaired_release(
        glib::clone!(@strong mouse_state, @strong cbs, @weak darea => move |_, _, _, d, _| {
            if mouse_state.borrow().pressing.get() == Some(d) {
                click_done_cb(&mouse_state, &darea, &cbs);
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
