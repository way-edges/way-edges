use crate::config::Config;
use crate::ui::draws::transition_state::TransitionState;

use super::BtnConfig;
use super::EventMap;
use clap::error::Result;
use gtk::cairo;
use gtk::cairo::Context;
use gtk::cairo::RectangleInt;
use gtk::cairo::Region;
use gtk::gdk::RGBA;
use gtk::glib;
use gtk::prelude::*;
use gtk::EventControllerMotion;
use gtk::{DrawingArea, GestureClick};
use gtk4_layer_shell::Edge;
use interval_task::runner;
use interval_task::runner::ExternalRunnerExt;
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

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
    pub fn take_pressing(&mut self) -> Option<u32> {
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
    fn start(&mut self, darea: &DrawingArea) -> Result<(), String> {
        if self.runner.is_none() {
            let (r, mut runner) = interval_task::channel::new(self.frame_gap);
            runner.start()?;
            self.runner = Some(runner);
            glib::spawn_future_local(glib::clone!(@weak darea => async move {
                while r.recv().await.is_ok() {
                    darea.queue_draw();
                }
            }));
        }
        Ok(())
    }
    fn stop(&mut self) -> Result<(), String> {
        if let Some(runner) = self.runner.take() {
            runner.close()?;
        }
        Ok(())
    }
}

pub fn setup_draw(
    window: &gtk::ApplicationWindow,
    cfg: Config,
    mut btn_cfg: BtnConfig,
) -> Result<DrawingArea, String> {
    let darea = DrawingArea::new();
    let size = cfg.get_size_into()?;
    let edge = cfg.edge;
    let extra_trigger_size = btn_cfg.extra_trigger_size.get_num_into()?;
    let map_size = ((size.0 + extra_trigger_size) as i32, size.1 as i32);
    match edge {
        Edge::Left | Edge::Right => {
            darea.set_width_request(map_size.0);
            darea.set_height_request(map_size.1);
        }
        Edge::Top | Edge::Bottom => {
            darea.set_width_request(map_size.1);
            darea.set_height_request(map_size.0);
        }
        _ => unreachable!(),
    };

    let transition_range = (0., size.0);
    let ts = TransitionState::new(
        Duration::from_millis(btn_cfg.transition_duration),
        transition_range.0,
        transition_range.1,
    );
    let mouse_state = MouseState::new(&ts);
    let is_pressing = mouse_state.pressing.clone();
    let set_rotate = draw_rotation(edge, size);
    let mut set_motion = draw_motion(edge, transition_range, extra_trigger_size);
    let set_core = draw_core(map_size, size, btn_cfg.color, extra_trigger_size)?;
    let set_input_region = draw_input_region(size, edge, extra_trigger_size);
    let mut set_frame_manger = draw_frame_manager(btn_cfg.frame_rate, transition_range);
    darea.set_draw_func(glib::clone!(@weak window =>move |darea, context, _, _| {
        set_rotate(context);
        let visible_y = ts.get_y();
        set_motion(context, visible_y);
        let res = set_core(context, is_pressing.get().is_some()).and_then(|_| {
            set_input_region(&window, visible_y).and_then(|_| {
                set_frame_manger(darea, visible_y, ts.is_forward.get())
            })
        });
        if let Err(e) = res {
            window.close();
            log::error!("{e}");
            // error ignored
            notify_rust::Notification::new().summary("Way-edges widget draw error").body(&e).show().ok();
        }
    }));
    let mouse_state = Rc::new(RefCell::new(mouse_state));
    set_event_mouse_click(
        &darea,
        btn_cfg.event_map.take().ok_or("EventMap is None")?,
        mouse_state.clone(),
    );
    set_event_mouse_move(&darea, mouse_state);
    window.set_child(Some(&darea));
    Ok(darea)
}

fn draw_core(
    map_size: (i32, i32),
    size: (f64, f64),
    color: RGBA,
    extra_trigger_size: f64,
) -> Result<impl Fn(&Context, bool) -> Result<(), String>, String> {
    let (b, n, p) = super::pre_draw::draw_to_surface(map_size, size, color, extra_trigger_size)?;
    let f_map_size = (map_size.0 as f64, map_size.1 as f64);

    fn error_handle(e: cairo::Error) -> String {
        format!("Draw core error: {:?}", e)
    }

    Ok(move |ctx: &Context, pressing: bool| -> Result<(), String> {
        // base_surface
        ctx.set_source_surface(&b, 0., 0.).map_err(error_handle)?;
        ctx.rectangle(0., 0., f_map_size.0, f_map_size.1);
        ctx.fill().map_err(error_handle)?;

        // mask
        if pressing {
            ctx.set_source_surface(&p, 0., 0.)
        } else {
            ctx.set_source_surface(&n, 0., 0.)
        }
        .map_err(error_handle)?;
        ctx.rectangle(0., 0., f_map_size.0, f_map_size.1);
        ctx.fill().map_err(error_handle)?;
        Ok(())
    })
}

fn draw_motion(
    edge: Edge,
    range: (f64, f64),
    extra_trigger_size: f64,
) -> impl FnMut(&Context, f64) {
    let offset: f64 = match edge {
        Edge::Right | Edge::Bottom => extra_trigger_size,
        _ => 0.,
    };
    move |ctx: &Context, visible_y: f64| {
        ctx.translate(-range.1 + visible_y - offset, 0.);
        // ctx.translate(range.1 - visible_y, 0.);
    }
}

fn draw_frame_manager(
    frame_rate: u64,
    range: (f64, f64),
) -> impl FnMut(&DrawingArea, f64, bool) -> Result<(), String> {
    let mut frame_manager = FrameManager::new(frame_rate);
    move |darea: &DrawingArea, visible_y: f64, is_forward: bool| {
        if (is_forward && visible_y < range.1) || (!is_forward && visible_y > range.0) {
            frame_manager.start(darea)?;
        } else {
            frame_manager.stop()?;
        }
        Ok(())
    }
}

fn draw_input_region(
    size: (f64, f64),
    edge: Edge,
    extra_trigger_size: f64,
) -> impl Fn(&gtk::ApplicationWindow, f64) -> Result<(), String> {
    let get_region: Box<dyn Fn(f64) -> Region> = match edge {
        Edge::Left => Box::new(move |visible_y: f64| {
            Region::create_rectangle(&RectangleInt::new(
                0,
                0,
                (visible_y + extra_trigger_size) as i32,
                size.1 as i32,
            ))
        }),
        Edge::Right => Box::new(move |visible_y: f64| {
            Region::create_rectangle(&RectangleInt::new(
                (size.0 - visible_y) as i32,
                0,
                (visible_y + extra_trigger_size).ceil() as i32,
                size.1 as i32,
            ))
        }),
        Edge::Top => Box::new(move |visible_y: f64| {
            Region::create_rectangle(&RectangleInt::new(
                0,
                0,
                size.1 as i32,
                (visible_y + extra_trigger_size) as i32,
            ))
        }),
        Edge::Bottom => Box::new(move |visible_y: f64| {
            Region::create_rectangle(&RectangleInt::new(
                0,
                (size.0 - visible_y) as i32,
                size.1 as i32,
                (visible_y + extra_trigger_size).ceil() as i32,
            ))
        }),
        _ => unreachable!(),
    };
    move |window: &gtk::ApplicationWindow, visible_y: f64| {
        window
            .surface()
            .ok_or("Input region surface not found")?
            .set_input_region(&get_region(visible_y));
        Ok(())
    }
}

fn draw_rotation(edge: Edge, size: (f64, f64)) -> Box<dyn Fn(&Context)> {
    match edge {
        Edge::Left => Box::new(move |_: &Context| {}),
        Edge::Right => Box::new(move |ctx: &Context| {
            ctx.rotate(180_f64.to_radians());
            ctx.translate(-size.0, -size.1);
        }),
        Edge::Top => Box::new(move |ctx: &Context| {
            ctx.rotate(90.0_f64.to_radians());
            ctx.translate(0., -size.1);
        }),
        Edge::Bottom => Box::new(move |ctx: &Context| {
            ctx.rotate(270.0_f64.to_radians());
            ctx.translate(-size.0, 0.);
        }),
        _ => unreachable!(),
    }
}

fn set_event_mouse_click(
    darea: &DrawingArea,
    event_map: EventMap,
    mouse_state: Rc<RefCell<MouseState>>,
) {
    let show_mouse_debug = crate::args::get_args().mouse_debug;
    let click_control = GestureClick::builder().button(0).exclusive(true).build();
    let cbs = Rc::new(RefCell::new(event_map));
    let click_done_cb = move |mouse_state: &Rc<RefCell<MouseState>>,
                              darea: &DrawingArea,
                              event_map: &Rc<RefCell<EventMap>>| {
        if let Some(btn) = mouse_state.borrow_mut().take_pressing() {
            if show_mouse_debug {
                notify(&format!("key released: {}", btn));
            };
            if let Some(cb) = event_map.borrow_mut().get_mut(&btn) {
                cb();
            };
            darea.queue_draw();
        } else {
            log::debug!("No pressing button in mouse_state");
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
