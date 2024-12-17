use std::cell::Cell;
use std::time::Duration;
use std::{cell::RefCell, rc::Rc};

use gtk::gdk::BUTTON_PRIMARY;
use gtk::prelude::WidgetExt;
use gtk::DrawingArea;
use gtk::{glib, ApplicationWindow};
use gtk4_layer_shell::Edge;

use crate::ui::draws::mouse_state::{
    new_mouse_event_func, new_mouse_state, MouseEvent, MouseState, MouseStateRc,
};
use crate::ui::draws::transition_state::TransitionStateRc;
use crate::ui::draws::util::Z;
use config::widgets::slide::{Direction, SlideConfig, Task};
use config::Config;

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
    window: &ApplicationWindow,
    darea: &DrawingArea,
    ts: TransitionStateRc,
    cfg: &Config,
    slide_cfg: &mut SlideConfig,
) -> (Rc<Cell<f64>>, MouseStateRc) {
    let xory = cfg.edge.into();
    let direction = slide_cfg.progress_direction;
    let max = slide_cfg.size().unwrap().1;
    let on_change = slide_cfg.on_change.take();
    let mut event_map = slide_cfg.event_map.take().unwrap();
    let update_with_interval_ms = slide_cfg.update_with_interval_ms.take();
    let draggable = slide_cfg.draggable;

    let ms = new_mouse_state(darea, MouseState::new(true, false, true, ts.clone()));
    let progress_state = Rc::new(RefCell::new(ProgressState::new(max, direction, on_change)));

    let cb = new_mouse_event_func(glib::clone!(
        #[weak]
        ms,
        #[weak]
        darea,
        #[strong]
        progress_state,
        move |e| {
            match e {
                MouseEvent::Release(_, k) => {
                    if let Some(f) = event_map.get_mut(&k) {
                        f()
                    }
                }
                MouseEvent::Press(pos, k) => {
                    if k == BUTTON_PRIMARY && draggable {
                        let progress = match xory {
                            XorY::X => pos.0,
                            XorY::Y => pos.1,
                        };
                        if !progress_state.borrow_mut().set_progress(progress) {
                            return;
                        }
                    }
                }
                MouseEvent::Motion(pos) => {
                    if draggable {
                        let is_middle = unsafe {
                            ms.as_ptr()
                                .as_ref()
                                .unwrap()
                                .pressing
                                .is_some_and(|k| k == BUTTON_PRIMARY)
                        };
                        if is_middle {
                            let progress = match xory {
                                XorY::X => pos.0,
                                XorY::Y => pos.1,
                            };
                            log::debug!("Change progress: {progress}");
                            if progress_state.borrow_mut().set_progress(progress) {
                                darea.queue_draw();
                            };
                        }
                    }
                }
                _ => {}
            };
            darea.queue_draw();
        }
    ));

    ms.borrow_mut().set_event_cb(cb);

    let progress = progress_state.borrow().current.clone();

    // update progress interval
    if let Some((ms, mut f)) = update_with_interval_ms {
        let (s, r) = async_channel::bounded::<f64>(1);
        let mut runner = interval_task::runner::new_runner(
            Duration::from_millis(ms),
            || (),
            move |_| {
                match f() {
                    Ok(p) => {
                        s.send_blocking(p).unwrap();
                    }
                    Err(e) => {
                        log::error!("Fail to get progress: {e}")
                    }
                };
                false
            },
        );
        if let Err(e) = runner.start() {
            log::error!("Failt to start runner for update interval: {e}");
        } else {
            glib::spawn_future_local(glib::clone!(
                #[strong]
                r,
                #[strong]
                progress,
                #[strong]
                darea,
                async move {
                    while let Ok(p) = r.recv().await {
                        progress.set(p);
                        darea.queue_draw();
                    }
                    log::warn!("progress update interval closed");
                }
            ));
            let runner = Rc::new(Cell::new(Some(runner)));
            window.connect_destroy(move |_| {
                log::debug!("closing slide progress update task");
                r.close();
                if let Some(runner) = runner.take() {
                    runner.close().unwrap();
                }
            });
        };
    };
    (progress, ms)
}
