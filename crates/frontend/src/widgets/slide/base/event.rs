use crate::mouse_state::MouseEvent;
use config::widgets::slide::base::SlideConfig;
use config::Config;
use gtk::gdk::BUTTON_PRIMARY;
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

pub fn setup_event(conf: &Config, w_conf: &mut SlideConfig) -> ProgressState {
    let func = make_translate_func(conf.edge);
    let left_pressing = false;

    ProgressState {
        left_pressing,
        length: w_conf.size().unwrap().1 as i32 - 2 * w_conf.border_width,
        border_width: w_conf.border_width,
        func,
    }
}
pub struct ProgressState {
    left_pressing: bool,

    length: i32,
    border_width: i32,
    func: fn(i32, i32, (f64, f64)) -> f64,
}
impl ProgressState {
    pub fn if_change_progress(&mut self, event: MouseEvent) -> Option<f64> {
        let mut p = None;
        match event {
            MouseEvent::Press(pos, key) => {
                if key == BUTTON_PRIMARY {
                    self.left_pressing = true;
                    p = Some((self.func)(self.length, self.border_width, pos))
                }
            }
            MouseEvent::Release(_, key) => {
                if key == BUTTON_PRIMARY {
                    self.left_pressing = false;
                }
            }
            MouseEvent::Motion(pos) => {
                if self.left_pressing {
                    p = Some((self.func)(self.length, self.border_width, pos))
                }
            }
            _ => {}
        }
        p
    }
}
