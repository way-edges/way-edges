use config::common::NumOrRelative;
use smithay_client_toolkit::shell::wlr_layer::Anchor;

#[derive(Debug)]
pub struct DrawCore {
    extra_trigger_size: i32,
    preview_size: NumOrRelative,

    visible_y_func: VisibleYFunc,
    pop_coordinate_func: PopCoordinateFunc,
}
impl DrawCore {
    pub fn new(conf: &config::Config) -> Self {
        let visible_y_func = make_visible_y_func(conf.edge);
        let pop_coordinate_func = make_pop_coordiante_pose_func(conf.edge);
        Self {
            extra_trigger_size: conf.extra_trigger_size.get_num().unwrap() as i32,
            preview_size: conf.preview_size,
            visible_y_func,
            pop_coordinate_func,
        }
    }
    pub fn calc_coordinate(&self, content_size: (i32, i32), progress: f64) -> [i32; 4] {
        let visible = (self.visible_y_func)(content_size, progress, self.preview_size);
        (self.pop_coordinate_func)(content_size, visible, self.extra_trigger_size)
    }
}

/// in: content_size, visible_y, preview_size
/// out: coordinate to translate, is will be <=0, size revealed
type VisibleYFunc = fn((i32, i32), f64, NumOrRelative) -> i32;

fn make_visible_y_func(edge: Anchor) -> VisibleYFunc {
    macro_rules! cal_pre {
        ($s:expr, $p:expr) => {
            match $p {
                NumOrRelative::Num(n) => n.ceil(),
                NumOrRelative::Relative(r) => ($s as f64 * r).ceil(),
            } as i32
        };
    }

    macro_rules! a {
        ($n:ident, $t:tt) => {
            fn $n(size: (i32, i32), ts_y: f64, preview: NumOrRelative) -> i32 {
                let preview = cal_pre!(size.$t, preview);
                let progress = (size.$t as f64 * ts_y).ceil() as i32;
                preview.max(progress)
            }
        };
    }

    a!(h, 0);
    a!(v, 1);

    match edge {
        Anchor::LEFT | Anchor::RIGHT => h,
        Anchor::TOP | Anchor::BOTTOM => v,
        _ => unreachable!(),
    }
}

/// in: content_size, visible_y, extra
/// out: coordinate to translate, is will be <=0, size revealed
type PopCoordinateFunc = fn((i32, i32), i32, i32) -> [i32; 4];

fn make_pop_coordiante_pose_func(edge: Anchor) -> PopCoordinateFunc {
    fn top(size: (i32, i32), visible_y: i32, extra: i32) -> [i32; 4] {
        let x = 0;
        let y = size.1 - visible_y;
        let w = size.0;
        let h = visible_y + extra;
        [x, y, w, h]
    }
    fn bottom(size: (i32, i32), visible_y: i32, extra: i32) -> [i32; 4] {
        let x = 0;
        let y = extra;
        let w = size.0;
        let h = visible_y + extra;
        [x, y, w, h]
    }
    fn left(size: (i32, i32), visible_y: i32, extra: i32) -> [i32; 4] {
        let x = size.0 - visible_y;
        let y = 0;
        let w = visible_y + extra;
        let h = size.1;
        [x, y, w, h]
    }
    fn right(size: (i32, i32), visible_y: i32, extra: i32) -> [i32; 4] {
        let x = extra;
        let y = 0;
        let w = visible_y + extra;
        let h = size.1;
        [x, y, w, h]
    }

    match edge {
        Anchor::LEFT => left,
        Anchor::TOP => top,
        Anchor::RIGHT => right,
        Anchor::BOTTOM => bottom,
        _ => unreachable!(),
    }
}
