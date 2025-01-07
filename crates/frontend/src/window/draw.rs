use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use cairo::{RectangleInt, Region};
use config::common::NumOrRelative;
use config::Config;
use gtk::prelude::{DrawingAreaExt, DrawingAreaExtManual, NativeExt, SurfaceExt};
use gtk::{glib, ApplicationWindow, DrawingArea};
use gtk4_layer_shell::Edge;
use util::Z;

use crate::animation::ToggleAnimationRc;
use crate::buffer::Buffer;

use super::frame::WindowFrameManager;
use super::WidgetContext;

fn set_area_size(darea: &DrawingArea, size: (i32, i32)) {
    darea.set_content_width(size.0);
    darea.set_content_height(size.1);
    // darea.set_size_request(new_size.0, new_size.1);
}

#[allow(clippy::too_many_arguments)]
pub fn set_draw_func(
    darea: &DrawingArea,
    window: &ApplicationWindow,
    start_pos: &Rc<Cell<(i32, i32)>>,
    pop_animation: &ToggleAnimationRc,

    widget: Weak<RefCell<dyn WidgetContext>>,
    has_update: Rc<Cell<bool>>,
    mut frame_manager: WindowFrameManager,
    mut buffer: Buffer,
    base_draw_func: impl Fn(&ApplicationWindow, &cairo::Context, (i32, i32), (i32, i32), f64) -> [i32; 4]
        + 'static,
    max_size_func: impl Fn((i32, i32)) -> (i32, i32) + 'static,
) {
    darea.set_draw_func(glib::clone!(
        #[weak]
        window,
        #[weak]
        start_pos,
        #[weak]
        pop_animation,
        move |darea: &DrawingArea, ctx: &cairo::Context, w, h| {
            // check unfinished animation and redraw frame
            let widget_has_animation_update = frame_manager.ensure_animations(darea);

            // content
            if has_update.get() || widget_has_animation_update {
                has_update.set(false);
                if let Some(w) = widget.upgrade() {
                    let img = w.borrow_mut().redraw();
                    let size = max_size_func((img.width(), img.height()));
                    set_area_size(darea, size);
                    buffer.update_buffer(img);
                }
            }
            let content = buffer.get_buffer();
            let content_size = (content.width(), content.height());
            let area_size = (w, h);

            // pop animation
            let progress = pop_animation.borrow_mut().progress();

            // input area && pop progress
            let pose = base_draw_func(&window, ctx, area_size, content_size, progress);
            start_pos.replace((pose[0], pose[1]));

            ctx.set_source_surface(content, Z, Z).unwrap();
            ctx.paint().unwrap();
        }
    ));
}

pub fn make_max_size_func(edge: Edge, extra: i32) -> impl Fn((i32, i32)) -> (i32, i32) {
    macro_rules! what_extra {
        ($size:expr, $extra:expr; H) => {
            $size.0 += $extra
        };
        ($size:expr, $extra:expr; V) => {{
            $size.1 += $extra
        }};
    }
    macro_rules! create_max_size_func {
        ($fn_name:ident, $index:tt) => {
            fn $fn_name(content_size: (i32, i32), extra: i32) -> (i32, i32) {
            let mut new = content_size;
                what_extra!(&mut new, extra; $index);
            new
            }
        };
    }
    create_max_size_func!(horizon, H);
    create_max_size_func!(vertical, V);
    let func = match edge {
        Edge::Left | Edge::Right => horizon,
        Edge::Top | Edge::Bottom => vertical,
        _ => unreachable!(),
    };

    move |size| func(size, extra)
}

#[allow(clippy::type_complexity)]
pub fn make_base_draw_func(
    conf: &Config,
) -> impl Fn(&gtk::ApplicationWindow, &cairo::Context, (i32, i32), (i32, i32), f64) -> [i32; 4] {
    let edge = conf.edge;
    let position = conf.position;
    let extra = conf.extra_trigger_size.get_num_into().unwrap().ceil() as i32;
    let preview = conf.preview_size;

    let visible_y_func = get_visible_y_func(edge);
    let xy_func = get_xy_func(edge, position);
    let preview_func = get_preview_size_func(edge, preview);
    let inr_func = get_input_region_func(edge, extra);

    move |window, ctx, area_size, content_size, animation_progress| {
        let visible_y = visible_y_func(content_size, animation_progress);
        let [x, y] = xy_func(area_size, content_size, visible_y);
        let mut pose = [x, y, content_size.0, content_size.1];

        preview_func(area_size, &mut pose);

        // input region
        if let Some(surf) = window.surface() {
            let inr = inr_func(pose);
            surf.set_input_region(&Region::create_rectangle(&inr));
        }

        // pop in progress
        ctx.translate(pose[0] as f64, pose[1] as f64);

        pose
    }
}

fn get_preview_size_func(edge: Edge, preview: NumOrRelative) -> impl Fn((i32, i32), &mut [i32; 4]) {
    macro_rules! cal_pre {
        ($s:expr, $p:expr) => {
            match $p {
                NumOrRelative::Num(n) => n.ceil(),
                NumOrRelative::Relative(r) => ($s as f64 * r).ceil(),
            } as i32
        };
    }
    macro_rules! edge_wh {
        ($area_size:expr, $size:expr, $p:expr; L) => {{
            let min = cal_pre!($size[2], $p);
            let n = $size[0];
            let l = $size[2];
            if n + l < min {
                $size[0] = -l + min
            }
        }};
        ($area_size:expr, $size:expr, $p:expr; R) => {{
            let min = cal_pre!($size[2], $p);
            let n = $size[0];
            let l = $area_size.0;
            if l - n < min {
                $size[0] = l - min
            }
        }};
        ($area_size:expr, $size:expr, $p:expr; T) => {{
            let min = cal_pre!($size[3], $p);
            let n = $size[1];
            let l = $size[3];
            if n + l < min {
                $size[1] = -l + min
            }
        }};
        ($area_size:expr, $size:expr, $p:expr; B) => {{
            let min = cal_pre!($size[3], $p);
            let n = $size[1];
            let l = $area_size.1;
            if l - n < min {
                $size[1] = l - min
            }
        }};
    }
    macro_rules! create_preview_fn {
        ($fn_name:ident, $index:tt) => {
            #[allow(unused_variables)]
            fn $fn_name(area_size: (i32, i32), pose: &mut [i32; 4], preview: NumOrRelative) {
                edge_wh!(area_size, pose, preview; $index)
            }
        };
    }
    create_preview_fn!(preview_left, L);
    create_preview_fn!(preview_right, R);
    create_preview_fn!(preview_top, T);
    create_preview_fn!(preview_bottom, B);
    let func = match edge {
        Edge::Left => preview_left,
        Edge::Right => preview_right,
        Edge::Top => preview_top,
        Edge::Bottom => preview_bottom,
        _ => unreachable!(),
    };

    move |area_size, pose| func(area_size, pose, preview)
}

fn get_visible_y_func(edge: Edge) -> fn((i32, i32), f64) -> i32 {
    macro_rules! edge_wh {
        ($size:expr, $ts_y:expr; H) => {
            ($size.0 as f64 * $ts_y).ceil() as i32
        };
        ($size:expr, $ts_y:expr; V) => {
            ($size.1 as f64 * $ts_y).ceil() as i32
        };
    }

    macro_rules! create_range_fn {
        ($fn_name:ident, $index:tt) => {
            fn $fn_name(content_size: (i32, i32), ts_y: f64) -> i32 {
                edge_wh!(content_size, ts_y; $index)
            }
        };
    }
    create_range_fn!(content_width, H);
    create_range_fn!(content_height, V);
    match edge {
        Edge::Left | Edge::Right => content_width,
        Edge::Top | Edge::Bottom => content_height,
        _ => unreachable!(),
    }
}

#[allow(clippy::type_complexity)]
fn get_xy_func(edge: Edge, position: Edge) -> fn((i32, i32), (i32, i32), i32) -> [i32; 2] {
    macro_rules! match_x {
        // position left
        ($i:ident, $area_size:expr, $content_size:expr, $visible_y:expr; POSITION_LEFT) => {
            let $i = 0;
        };
        // position middle
        ($i:ident, $area_size:expr, $content_size:expr, $visible_y:expr; POSITION_CENTER) => {
            let $i = (calculate_x_additional($area_size.0, $content_size.0) / 2);
        };
        // position right
        ($i:ident, $area_size:expr, $content_size:expr, $visible_y:expr; POSITION_RIGHT) => {
            let $i = calculate_x_additional($area_size.0, $content_size.0);
        };
        // edge left
        ($i:ident, $area_size:expr, $content_size:expr, $visible_y:expr; EDGE_LEFT) => {
            let $i = (-$content_size.0 + $visible_y);
        };
        // edge right
        ($i:ident, $area_size:expr, $content_size:expr, $visible_y:expr; EDGE_RIGHT) => {
            let a = calculate_x_additional($area_size.0, $content_size.0);
            let $i = ($content_size.0 - $visible_y) + a;
        };
    }
    macro_rules! match_y {
        // position top
        ($i:ident, $area_size:expr, $content_size:expr, $visible_y:expr; POSITION_TOP) => {
            let $i = 0;
        };
        // position middle
        ($i:ident, $area_size:expr, $content_size:expr, $visible_y:expr; POSITION_CENTER) => {
            let $i = (calculate_y_additional($area_size.1, $content_size.1) / 2);
        };
        // position bottom
        ($i:ident, $area_size:expr, $content_size:expr, $visible_y:expr; POSITION_BOTTOM) => {
            let $i = calculate_y_additional($area_size.1, $content_size.1);
        };
        // edge top
        ($i:ident, $area_size:expr, $content_size:expr, $visible_y:expr; EDGE_TOP) => {
            let $i = (-$content_size.1 + $visible_y);
        };
        // edge bottom
        ($i:ident, $area_size:expr, $content_size:expr, $visible_y:expr; EDGE_BOTTOM) => {
            let a = calculate_y_additional($area_size.1, $content_size.1);
            let $i = ($content_size.1 - $visible_y) + a;
        };
    }

    macro_rules! create_position_fn {
        ($fn_name:ident, $x_arg:tt, $y_arg:tt, $wh_arg:tt) => {
            #[allow(unused_variables)]
            fn $fn_name(
                area_size: (i32, i32),
                content_size: (i32, i32),
                visible_y: i32,
            )->[i32; 2] {
                match_x!(x, area_size, content_size, visible_y; $x_arg);
                match_y!(y, area_size, content_size, visible_y; $y_arg);
                [x, y]
            }
        };
    }

    create_position_fn!(left_center, EDGE_LEFT, POSITION_CENTER, H);
    create_position_fn!(left_top, EDGE_LEFT, POSITION_TOP, H);
    create_position_fn!(left_bottom, EDGE_LEFT, POSITION_BOTTOM, H);

    create_position_fn!(right_center, EDGE_RIGHT, POSITION_CENTER, H);
    create_position_fn!(right_top, EDGE_RIGHT, POSITION_TOP, H);
    create_position_fn!(right_bottom, EDGE_RIGHT, POSITION_BOTTOM, H);

    create_position_fn!(top_center, POSITION_CENTER, EDGE_TOP, V);
    create_position_fn!(top_left, POSITION_LEFT, EDGE_TOP, V);
    create_position_fn!(top_right, POSITION_RIGHT, EDGE_TOP, V);

    create_position_fn!(bottom_center, POSITION_CENTER, EDGE_BOTTOM, V);
    create_position_fn!(bottom_left, POSITION_LEFT, EDGE_BOTTOM, V);
    create_position_fn!(bottom_right, POSITION_RIGHT, EDGE_BOTTOM, V);

    match (edge, position) {
        // left center
        (Edge::Left, Edge::Left) | (Edge::Left, Edge::Right) => left_center,
        // left top
        (Edge::Left, Edge::Top) => left_top,
        // left bottom
        (Edge::Left, Edge::Bottom) => left_bottom,
        // right center
        (Edge::Right, Edge::Left) | (Edge::Right, Edge::Right) => right_center,
        // right top
        (Edge::Right, Edge::Top) => right_top,
        // right bottom
        (Edge::Right, Edge::Bottom) => right_bottom,
        // top center
        (Edge::Top, Edge::Top) | (Edge::Top, Edge::Bottom) => top_center,
        // top left
        (Edge::Top, Edge::Left) => top_left,
        // top right
        (Edge::Top, Edge::Right) => top_right,
        // bottom center
        (Edge::Bottom, Edge::Top) | (Edge::Bottom, Edge::Bottom) => bottom_center,
        // bottom left
        (Edge::Bottom, Edge::Left) => bottom_left,
        // bottom right
        (Edge::Bottom, Edge::Right) => bottom_right,
        _ => unreachable!(),
    }
}

fn get_input_region_func(edge: Edge, extra: i32) -> impl Fn([i32; 4]) -> RectangleInt {
    macro_rules! match_inr {
        ($l:expr, $extra:expr, TOP) => {
            $l[3] += $extra
        };
        ($l:expr, $extra:expr, BOTTOM) => {
            $l[1] -= $extra;
            $l[3] += $extra
        };
        ($l:expr, $extra:expr, LEFT) => {
            $l[2] += $extra
        };
        ($l:expr, $extra:expr, RIGHT) => {
            $l[0] -= $extra;
            $l[2] += $extra
        };
    }
    macro_rules! create_inr_fn {
        ($fn_name:ident, $b:tt) => {
            fn $fn_name(mut l: [i32; 4], extra: i32) -> RectangleInt {
                match_inr!(&mut l, extra, $b);
                RectangleInt::new(l[0], l[1], l[2], l[3])
            }
        };
    }
    create_inr_fn!(inr_top, TOP);
    create_inr_fn!(inr_bottom, BOTTOM);
    create_inr_fn!(inr_left, LEFT);
    create_inr_fn!(inr_right, RIGHT);

    let get_inr = match edge {
        Edge::Top => inr_top,
        Edge::Bottom => inr_bottom,
        Edge::Left => inr_left,
        Edge::Right => inr_right,
        _ => unreachable!(),
    };

    move |l| get_inr(l, extra)
}

fn calculate_x_additional(area_width: i32, content_width: i32) -> i32 {
    (area_width).max(content_width) - content_width
}
fn calculate_y_additional(area_height: i32, content_height: i32) -> i32 {
    (area_height).max(content_height) - content_height
}
