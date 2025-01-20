use config::common::NumOrRelative;
use smithay_client_toolkit::shell::wlr_layer::Anchor;

pub struct DrawCore {
    extra_trigger_size: i32,
    preview_size: NumOrRelative,

    /// arg: content_size, animation_progress(0-1)
    /// out: visible_y
    visible_y_func: fn((i32, i32), f64) -> i32,
    /// arg: area_size, content_size, visible_y
    /// out: x&y coordinate
    xy_coordinate_func: fn((i32, i32), (i32, i32), i32) -> [i32; 2],
    /// arg: area_size, pose, preview
    preview_size_func: fn((i32, i32), &mut [i32; 4], NumOrRelative),
    /// arg: pose, extra_trigger_size
    /// out: input region x,y,w,h
    input_region_func: fn([i32; 4], i32) -> [i32; 4],

    /// arg: content_size, extra_trigger_size
    /// out: area_size
    max_size_func: fn((i32, i32), i32) -> (i32, i32),
}
impl DrawCore {
    pub fn new(conf: &config::Config) -> Self {
        let visible_y_func = get_visible_y_func(conf.edge);
        let xy_coordinate_func = get_xy_func(conf.edge, conf.position);
        let preview_size_func = get_preview_size_func(conf.edge);
        let input_region_func = get_input_region_func(conf.edge);
        let max_size_func = make_max_size_func(conf.edge);
        Self {
            extra_trigger_size: conf.extra_trigger_size.get_num().unwrap() as i32,
            preview_size: conf.preview_size,
            visible_y_func,
            xy_coordinate_func,
            preview_size_func,
            input_region_func,
            max_size_func,
        }
    }
    pub fn draw_pop(
        &self,
        ctx: &cairo::Context,
        area_size: (i32, i32),
        content_size: (i32, i32),
        animation_progress: f64,
    ) -> [i32; 4] {
        let visible_y = (self.visible_y_func)(content_size, animation_progress);
        let [x, y] = (self.xy_coordinate_func)(area_size, content_size, visible_y);
        let mut pose = [x, y, content_size.0, content_size.1];

        (self.preview_size_func)(area_size, &mut pose, self.preview_size);

        // // input region
        // if let Some(surf) = window.surface() {
        //     let inr = inr_func(pose);
        //     surf.set_input_region(&Region::create_rectangle(&inr));
        // }

        // pop in progress
        ctx.translate(pose[0] as f64, pose[1] as f64);
        pose
    }
    pub fn calc_input_region(&self, pose: [i32; 4]) -> [i32; 4] {
        (self.input_region_func)(pose, self.extra_trigger_size)
    }
    pub fn calc_max_size(&self, content_size: (i32, i32)) -> (i32, i32) {
        (self.max_size_func)(content_size, self.extra_trigger_size)
    }
}

fn make_max_size_func(edge: Anchor) -> fn((i32, i32), i32) -> (i32, i32) {
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
    match edge {
        Anchor::LEFT | Anchor::RIGHT => horizon,
        Anchor::TOP | Anchor::BOTTOM => vertical,
        _ => unreachable!(),
    }
}

fn get_preview_size_func(edge: Anchor) -> fn((i32, i32), &mut [i32; 4], NumOrRelative) {
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
    match edge {
        Anchor::LEFT => preview_left,
        Anchor::RIGHT => preview_right,
        Anchor::TOP => preview_top,
        Anchor::BOTTOM => preview_bottom,
        _ => unreachable!(),
    }
}

fn get_visible_y_func(edge: Anchor) -> fn((i32, i32), f64) -> i32 {
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
        Anchor::LEFT | Anchor::RIGHT => content_width,
        Anchor::TOP | Anchor::BOTTOM => content_height,
        _ => unreachable!(),
    }
}

#[allow(clippy::type_complexity)]
fn get_xy_func(edge: Anchor, position: Anchor) -> fn((i32, i32), (i32, i32), i32) -> [i32; 2] {
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
        (Anchor::LEFT, Anchor::LEFT) | (Anchor::LEFT, Anchor::RIGHT) => left_center,
        // left top
        (Anchor::LEFT, Anchor::TOP) => left_top,
        // left bottom
        (Anchor::LEFT, Anchor::BOTTOM) => left_bottom,
        // right center
        (Anchor::RIGHT, Anchor::LEFT) | (Anchor::RIGHT, Anchor::RIGHT) => right_center,
        // right top
        (Anchor::RIGHT, Anchor::TOP) => right_top,
        // right bottom
        (Anchor::RIGHT, Anchor::BOTTOM) => right_bottom,
        // top center
        (Anchor::TOP, Anchor::TOP) | (Anchor::TOP, Anchor::BOTTOM) => top_center,
        // top left
        (Anchor::TOP, Anchor::LEFT) => top_left,
        // top right
        (Anchor::TOP, Anchor::RIGHT) => top_right,
        // bottom center
        (Anchor::BOTTOM, Anchor::TOP) | (Anchor::BOTTOM, Anchor::BOTTOM) => bottom_center,
        // bottom left
        (Anchor::BOTTOM, Anchor::LEFT) => bottom_left,
        // bottom right
        (Anchor::BOTTOM, Anchor::RIGHT) => bottom_right,
        _ => unreachable!(),
    }
}

fn get_input_region_func(edge: Anchor) -> fn([i32; 4], i32) -> [i32; 4] {
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
            fn $fn_name(mut l: [i32; 4], extra: i32) -> [i32; 4] {
                match_inr!(&mut l, extra, $b);
                l
            }
        };
    }
    create_inr_fn!(inr_top, TOP);
    create_inr_fn!(inr_bottom, BOTTOM);
    create_inr_fn!(inr_left, LEFT);
    create_inr_fn!(inr_right, RIGHT);

    match edge {
        Anchor::TOP => inr_top,
        Anchor::BOTTOM => inr_bottom,
        Anchor::LEFT => inr_left,
        Anchor::RIGHT => inr_right,
        _ => unreachable!(),
    }
}

fn calculate_x_additional(area_width: i32, content_width: i32) -> i32 {
    (area_width).max(content_width) - content_width
}
fn calculate_y_additional(area_height: i32, content_height: i32) -> i32 {
    (area_height).max(content_height) - content_height
}
