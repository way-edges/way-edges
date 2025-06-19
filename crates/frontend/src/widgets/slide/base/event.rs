use std::cell::Cell;
use std::rc::Rc;

use crate::mouse_state::MouseEvent;
use config::widgets::slide::base::SlideConfig;
use smithay_client_toolkit::seat::pointer::BTN_LEFT;
use smithay_client_toolkit::shell::wlr_layer::Anchor;

fn make_translate_func(edge: Anchor) -> fn(i32, i32, (f64, f64)) -> f64 {
    macro_rules! hhh {
        ($len:expr, $border_width:expr, $pos:expr, VERTICAL) => {
            (($len as f64 - ($pos.1 - $border_width as f64)) / $len as f64)
                .min(1.)
                .max(0.)
        };
        ($len:expr, $border_width:expr,  $pos:expr, HORIZONTAL) => {
            (($pos.0 - $border_width as f64) / $len as f64)
                .min(1.)
                .max(0.)
        };
    }

    macro_rules! create_func {
        ($name:ident, $i:tt) => {
            fn $name(length: i32, border_width: i32, pos: (f64, f64)) -> f64 {
                hhh!(length, border_width, pos, $i)
            }
        };
    }

    create_func!(lor, VERTICAL);
    create_func!(tob, HORIZONTAL);

    match edge {
        Anchor::LEFT | Anchor::RIGHT => lor,
        Anchor::TOP | Anchor::BOTTOM => tob,
        _ => unreachable!(),
    }
}

pub trait ProgressData {
    fn get(&self) -> f64;
    fn set(&mut self, value: f64);
}

pub fn setup_event<T: ProgressData>(
    edge: Anchor,
    w_conf: &SlideConfig,
    data: T,
) -> ProgressState<T> {
    let func = make_translate_func(edge);
    let left_pressing = false;

    ProgressState {
        left_pressing,
        scroll_unit: w_conf.scroll_unit,
        length: w_conf.size().unwrap().1 as i32 - 2 * w_conf.border_width,
        border_width: w_conf.border_width,
        func,
        progress: data,
    }
}

pub type ProgressDataf = Rc<Cell<f64>>;
impl ProgressData for ProgressDataf {
    fn get(&self) -> f64 {
        Cell::get(self)
    }

    fn set(&mut self, value: f64) {
        Cell::set(self, value);
    }
}

#[derive(Debug)]
pub struct ProgressState<T: ProgressData> {
    left_pressing: bool,
    length: i32,
    border_width: i32,
    func: fn(i32, i32, (f64, f64)) -> f64,
    scroll_unit: f64,

    progress: T,
}
impl<T: ProgressData> ProgressState<T> {
    fn calculate(&self, pos: (f64, f64)) -> f64 {
        (self.func)(self.length, self.border_width, pos)
    }
    pub fn p(&self) -> f64 {
        self.progress.get()
    }
    pub fn data(&mut self) -> &mut T {
        &mut self.progress
    }
    pub fn if_change_progress(
        &mut self,
        event: MouseEvent,
        update_progress_immediate: bool,
    ) -> Option<f64> {
        let mut p = None;
        match event {
            MouseEvent::Press(pos, key) => {
                if key == BTN_LEFT {
                    self.left_pressing = true;
                    p = Some(self.calculate(pos));
                }
            }
            MouseEvent::Release(pos, key) => {
                if key == BTN_LEFT {
                    self.left_pressing = false;
                    p = Some(self.calculate(pos));
                }
            }
            MouseEvent::Motion(pos) => {
                if self.left_pressing {
                    p = Some(self.calculate(pos));
                }
            }
            MouseEvent::Scroll(_, v) => {
                p = Some((self.progress.get() + (self.scroll_unit * v.absolute)).clamp(0.0, 1.0));
            }
            _ => {}
        }

        #[allow(clippy::unnecessary_unwrap)]
        if update_progress_immediate && p.is_some() {
            self.progress.set(p.unwrap());
        }

        p
    }
}
