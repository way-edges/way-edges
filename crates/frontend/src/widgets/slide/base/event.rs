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

    let func = match edge {
        Edge::Left | Edge::Right => lor,
        Edge::Top | Edge::Bottom => tob,
        _ => unreachable!(),
    };

    let length = w_conf.size().unwrap().1 as i32 - 2 * w_conf.border_width;
    let border_width = w_conf.border_width;

    move |pos| func(length, border_width, pos)
}

pub fn setup_event(
    window: &mut WindowContext,
    conf: &Config,
    w_conf: &mut SlideConfig,
    mut key_callback: Option<impl FnMut(u32) + 'static>,
    mut set_progress_callback: impl FnMut(f64) + 'static,
    draw_func: Option<Rc<impl 'static + Fn(f64) -> ImageSurface>>,
) {
    let progress_func = make_translate_func(w_conf, conf.edge);
    let trigger_redraw = window.make_redraw_notifier();
    let not_do_redraw = w_conf.redraw_only_on_internal_update;
    let mut do_progress_func = move |pos| {
        let progress = progress_func(pos);
        set_progress_callback(progress);
        if !not_do_redraw {
            if let Some(draw_func) = draw_func.as_ref() {
                trigger_redraw(Some(draw_func(progress)));
            }
        }
    };

    let mut left_pressing = false;
    window.setup_mouse_event_callback(move |_, event| {
        match event {
            MouseEvent::Press(pos, key) => {
                if key == BUTTON_PRIMARY {
                    left_pressing = true;
                    do_progress_func(pos);
                }
            }
            MouseEvent::Release(_, key) => {
                if key == BUTTON_PRIMARY {
                    left_pressing = false;
                }
                if let Some(key_callback) = &mut key_callback {
                    key_callback(key);
                }
            }
            MouseEvent::Motion(pos) => {
                if left_pressing {
                    do_progress_func(pos);
                }
            }
            _ => {}
        }

        false
    });
}
