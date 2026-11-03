use config::def::{common::CommonConfig, shared::NumOrRelative};
use smithay_client_toolkit::shell::wlr_layer::Anchor;

#[derive(Debug)]
pub struct DrawCore {
    extra_trigger_size: i32,
    preview_size: NumOrRelative,

    visible_y_func: VisibleYFunc,
    pop_coordinate_func: PopCoordinateFunc,
}
impl DrawCore {
    pub fn new(conf: &CommonConfig) -> Self {
        let visible_y_func = make_visible_y_func(conf.edge);
        let pop_coordinate_func = make_pop_coordiante_pose_func(conf.edge);
        Self {
            extra_trigger_size: conf.extra_trigger_size.get_num().unwrap() as i32,
            preview_size: conf.preview_size,
            visible_y_func,
            pop_coordinate_func,
        }
    }
    pub fn calc_coordinate(
        &self,
        content_size: (i32, i32),
        offset: i32,
        progress: f64,
    ) -> [i32; 4] {
        let visible = (self.visible_y_func)(content_size, offset, progress, self.preview_size);
        (self.pop_coordinate_func)(content_size, visible, self.extra_trigger_size)
    }
}

/// in: content_size, offset, visible_y, preview_size
/// out: coordinate to translate, is will be <=0, size revealed
type VisibleYFunc = fn((i32, i32), i32, f64, NumOrRelative) -> i32;

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
            fn $n(size: (i32, i32), offset: i32, progress: f64, preview: NumOrRelative) -> i32 {
                let preview = cal_pre!(size.$t, preview);
                let total = size.$t + offset;
                let visable = (total as f64 * progress).ceil() as i32;
                visable.max(preview)
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
        let y = visible_y - size.1;
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
        let x = visible_y - size.0;
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

#[cfg(test)]
mod tests {
    use super::*;
    use config::def::WidgetConf;

    #[test]
    fn side_docks_hide_and_reveal_across_the_horizontal_axis() {
        for (source, edge) in [
            (
                include_str!("../../../../doc/config/dock-left.kdl"),
                Anchor::LEFT,
            ),
            (
                include_str!("../../../../doc/config/dock-right.kdl"),
                Anchor::RIGHT,
            ),
        ] {
            let root = config::def::parse_kdl(source).unwrap();
            let WidgetConf::Dock(dock) = &root.widgets[0] else {
                panic!("Expected dock")
            };
            assert_eq!(dock.common.edge, edge);
            assert!(!dock.common.pin_on_startup);
            assert_eq!(dock.widget.pop_duration, 1500);
            let draw = DrawCore::new(&dock.common);
            let size = (400, 800);
            let extra = dock.common.extra_trigger_size.get_num().unwrap() as i32;
            for (progress, visible) in [(0.0, 0), (0.5, 200), (1.0, 400)] {
                let [x, y, width, height] = draw.calc_coordinate(size, 0, progress);
                assert_eq!(y, 0);
                assert_eq!(height, size.1);
                assert_eq!(width, visible + extra);
                assert_eq!(
                    x,
                    if edge == Anchor::LEFT {
                        visible - size.0
                    } else {
                        extra
                    }
                );
                let visible_content = (x + size.0).min(width) - x.max(0);
                assert_eq!(visible_content.max(0), visible);
            }
        }
    }
}
