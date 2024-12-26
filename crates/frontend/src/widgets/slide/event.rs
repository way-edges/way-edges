use std::cell::Cell;
use std::rc::Rc;

use crate::mouse_state::MouseEvent;
use crate::window::WindowContext;
use cairo::ImageSurface;
use config::widgets::slide::base::SlideConfig;
use config::Config;
use gtk::gdk::BUTTON_PRIMARY;
use gtk4_layer_shell::Edge;

fn make_translate_func(w_conf: &SlideConfig, edge: Edge) -> impl 'static + Fn((f64, f64)) -> f64 {
    macro_rules! hhh {
        ($size:expr, $pos:expr, VERTICAL) => {
            ($pos.1 / $size.1 as f64).min(1.).max(0.)
        };
        ($size:expr, $pos:expr, HORIZONTAL) => {
            ($pos.0 / $size.0 as f64).min(1.).max(0.)
        };
    }

    macro_rules! create_func {
        ($name:ident, $i:tt) => {
            fn $name(size: (i32, i32), pos: (f64, f64)) -> f64 {
                hhh!(size, pos, $i)
            }
        };
    }

    create_func!(lor, VERTICAL);
    create_func!(tob, HORIZONTAL);

    let func = match edge {
        Edge::Left | Edge::Right => lor,
        Edge::Top | Edge::Bottom => tob,
        _ => unreachable!(),
    };

    let size = w_conf.size().unwrap();
    let size = match edge {
        Edge::Left | Edge::Right => (size.0 as i32, size.1 as i32),
        Edge::Top | Edge::Bottom => (size.1 as i32, size.0 as i32),
        _ => unreachable!(),
    };

    move |pos| func(size, pos)
}

pub(super) fn setup_event(
    window: &mut WindowContext,
    conf: &Config,
    w_conf: &mut SlideConfig,
    draw_func: Rc<impl 'static + Fn(f64) -> ImageSurface>,
    progress_cache: Rc<Cell<f64>>,
) {
    let mut event_map = std::mem::take(&mut w_conf.event_map);
    let trigger_redraw = window.make_redraw_notifier();
    let mut left_pressing = false;
    let progress_func = make_translate_func(w_conf, conf.edge);

    window.setup_mouse_event_callback(move |_, event| {
        match event {
            MouseEvent::Press(pos, key) => {
                if key == BUTTON_PRIMARY {
                    left_pressing = true;
                    let progress = progress_func(pos);
                    progress_cache.set(progress);
                    trigger_redraw(Some(draw_func(progress)));
                }
            }
            MouseEvent::Release(_, key) => {
                if key == BUTTON_PRIMARY {
                    left_pressing = false;
                }
                if let Some(cb) = event_map.get_mut(&key) {
                    cb();
                };
            }
            MouseEvent::Motion(pos) => {
                if left_pressing {
                    let progress = progress_func(pos);
                    progress_cache.set(progress);
                    trigger_redraw(Some(draw_func(progress)));
                }
            }
            _ => {}
        }

        false
    });
}
