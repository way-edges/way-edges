use std::rc::Rc;

use cairo::{ImageSurface, RectangleInt, Region};
use gtk::prelude::{NativeExt, SurfaceExt};
use gtk4_layer_shell::Edge;

use crate::animation;

use super::_WindowContext;

impl _WindowContext {
    fn set_draw_func(&self, cb: impl 'static + FnMut() -> ImageSurface) {
        self.drawing_area.set_draw_func(cb);
    }
}

pub type DrawMotionFunc = Rc<dyn Fn(&cairo::Context, (i32, i32), (i32, i32), f64)>;
//                                            area-size  content_size progress
pub fn make_motion_func(edge: Edge, position: Edge) -> DrawMotionFunc {
    // NOTE: WE NEED BETTER CODE FOR THIS.

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
            let $i = (-$content_size.0 + $visible_y as i32);
        };
        // edge right
        ($i:ident, $area_size:expr, $content_size:expr, $visible_y:expr; EDGE_RIGHT) => {
            let a = calculate_x_additional($area_size.0, $content_size.0);
            let $i = ($content_size.0 - $visible_y as i32) + a;
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
            let $i = (-$content_size.1 + $visible_y as i32);
        };
        // edge bottom
        ($i:ident, $area_size:expr, $content_size:expr, $visible_y:expr; EDGE_BOTTOM) => {
            let a = calculate_y_additional($area_size.1, $content_size.1);
            let $i = ($content_size.1 - $visible_y as i32) + a;
        };
    }
    macro_rules! create_position_fn {
        ($fn_name:ident, $x_arg:tt, $y_arg:tt) => {
            #[allow(unused_variables)]
            fn $fn_name(
                ctx: &cairo::Context,
                area_size: (i32, i32),
                content_size: (i32, i32),
                visible_y: f64,
            ) {
                match_x!(x, area_size, content_size, visible_y; $x_arg);
                match_y!(y, area_size, content_size, visible_y; $y_arg);
                ctx.translate(x as f64, y as f64)
            }
        };
    }
    create_position_fn!(left_center, EDGE_LEFT, POSITION_CENTER);
    create_position_fn!(left_top, EDGE_LEFT, POSITION_TOP);
    create_position_fn!(left_bottom, EDGE_LEFT, POSITION_BOTTOM);

    create_position_fn!(right_center, EDGE_RIGHT, POSITION_CENTER);
    create_position_fn!(right_top, EDGE_RIGHT, POSITION_TOP);
    create_position_fn!(right_bottom, EDGE_RIGHT, POSITION_BOTTOM);

    create_position_fn!(top_center, POSITION_CENTER, EDGE_TOP);
    create_position_fn!(top_left, POSITION_LEFT, EDGE_TOP);
    create_position_fn!(top_right, POSITION_RIGHT, EDGE_TOP);

    create_position_fn!(bottom_center, POSITION_CENTER, EDGE_BOTTOM);
    create_position_fn!(bottom_left, POSITION_LEFT, EDGE_BOTTOM);
    create_position_fn!(bottom_right, POSITION_RIGHT, EDGE_BOTTOM);

    // 12 different cases
    let draw_motion = match (edge, position) {
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
    };

    macro_rules! create_range_fn {
        ($fn_name:ident, $index:tt) => {
            #[allow(unused_variables)]
            fn $fn_name(content_size: (i32, i32)) -> i32 {
                content_size.$index
            }
        };
    }
    create_range_fn!(content_width, 0);
    create_range_fn!(content_height, 1);
    let get_range = match edge {
        Edge::Left | Edge::Right => content_width,
        Edge::Top | Edge::Bottom => content_height,
        _ => unreachable!(),
    };

    Rc::new(move |ctx, area_size, content_size, y| {
        let range = get_range(content_size);
        let visible_y = animation::calculate_transition(y, (0., range as f64));
        draw_motion(ctx, area_size, content_size, visible_y);
    })
}

pub type SetWindowInputRegionFunc =
    Rc<dyn Fn(&gtk::ApplicationWindow, (i32, i32), (i32, i32), f64) -> RectangleInt>;
pub fn make_window_input_region_fun(
    edge: Edge,
    position: Edge,
    extra_trigger_size: i32,
) -> SetWindowInputRegionFunc {
    // NOTE: WE NEED BETTER CODE FOR THIS.
    macro_rules! edge_wh {
        ($w:ident, $h:ident, $size:expr, $ts_y:expr; H) => {
            let $w = ($size.0 as f64 * $ts_y).ceil() as i32;
            let $h = $size.1;
        };
        ($w:ident, $h:ident, $size:expr, $ts_y:expr; V) => {
            let $w = $size.0;
            let $h = ($size.1 as f64 * $ts_y).ceil() as i32;
        };
    }
    macro_rules! match_x {
        // position left
        ($i:ident, $area_size:expr, $content_size:expr, $ts_y:expr; POSITION_LEFT) => {
            let $i = 0;
        };
        // position middle
        ($i:ident, $area_size:expr, $content_size:expr, $ts_y:expr; POSITION_CENTER) => {
            let $i = (calculate_x_additional($area_size.0, $content_size.0) / 2);
        };
        // position right
        ($i:ident, $area_size:expr, $content_size:expr, $ts_y:expr; POSITION_RIGHT) => {
            let $i = calculate_x_additional($area_size.0, $content_size.0);
        };
        // edge left
        ($i:ident, $area_size:expr, $content_size:expr, $ts_y:expr; EDGE_LEFT) => {
            let $i = 0;
        };
        // edge right
        ($i:ident, $area_size:expr, $content_size:expr, $ts_y:expr; EDGE_RIGHT) => {
            let $i = (($content_size.0 as f64) * (1. - $ts_y)) as i32
                + calculate_x_additional($area_size.0, $content_size.0);
        };
    }
    macro_rules! match_y {
        // position top
        ($i:ident, $area_size:expr, $content_size:expr, $ts_y:expr; POSITION_TOP) => {
            let $i = 0;
        };
        // position middle
        ($i:ident, $area_size:expr, $content_size:expr, $ts_y:expr; POSITION_CENTER) => {
            let $i = (calculate_y_additional($area_size.1, $content_size.1) / 2);
        };
        // position bottom
        ($i:ident, $area_size:expr, $content_size:expr, $ts_y:expr; POSITION_BOTTOM) => {
            let $i = calculate_y_additional($area_size.1, $content_size.1);
        };
        // edge top
        ($i:ident, $area_size:expr, $content_size:expr, $ts_y:expr; EDGE_TOP) => {
            let $i = 0;
        };
        // edge bottom
        ($i:ident, $area_size:expr, $content_size:expr, $ts_y:expr; EDGE_BOTTOM) => {
            let $i = (($content_size.1 as f64) * (1. - $ts_y)) as i32
                + calculate_y_additional($area_size.1, $content_size.1);
        };
    }

    macro_rules! create_position_fn {
        ($fn_name:ident, $x_arg:tt, $y_arg:tt, $wh_arg:tt) => {
            #[allow(unused_variables)]
            fn $fn_name(
                area_size: (i32, i32),
                content_size: (i32, i32),
                ts_y: f64,
            )->[i32; 4] {

                match_x!(x, area_size, content_size, ts_y; $x_arg);
                match_y!(y, area_size, content_size, ts_y; $y_arg);
                edge_wh!(w, h, content_size, ts_y; $wh_arg);
                [x, y, w, h]
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

    // 12 different cases
    let get_xywh = match (edge, position) {
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
    };

    macro_rules! match_inr {
        ($inr:expr, $extra:expr, TOP) => {
            $inr.set_height($inr.height() + $extra);
        };
        ($inr:expr, $extra:expr, BOTTOM) => {
            $inr.set_y($inr.y() - $extra);
            $inr.set_height($inr.height() + $extra);
        };
        ($inr:expr, $extra:expr, LEFT) => {
            $inr.set_width($inr.width() + $extra);
        };
        ($inr:expr, $extra:expr, RIGHT) => {
            $inr.set_x($inr.x() - $extra);
            $inr.set_width($inr.width() + $extra);
        };
    }
    macro_rules! create_inr_fn {
        ($fn_name:ident, $b:tt) => {
            #[allow(unused_variables)]
            fn $fn_name(inr: &mut RectangleInt, extra: i32) {
                match_inr!(inr, extra, $b);
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

    Rc::new(move |window, area_size, content_size, ts_y| {
        let [x, y, w, h] = get_xywh(area_size, content_size, ts_y);
        // box normal input region
        let rec_int = RectangleInt::new(x, y, w, h);

        {
            // box input region add extra_trigger
            let mut inr = rec_int;
            get_inr(&mut inr, extra_trigger_size);
            if let Some(surf) = window.surface() {
                surf.set_input_region(&Region::create_rectangle(&inr));
            }
        }

        rec_int
    })
}

fn calculate_x_additional(area_width: i32, content_width: i32) -> i32 {
    (area_width).max(content_width) - content_width
}
fn calculate_y_additional(area_height: i32, content_height: i32) -> i32 {
    (area_height).max(content_height) - content_height
}
