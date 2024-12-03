use std::time::Duration;

use cairo::{RectangleInt, Region};
use gtk::glib;
use gtk::prelude::NativeExt;
use gtk::prelude::{SurfaceExt, WidgetExt};
use gtk::DrawingArea;
use gtk4_layer_shell::Edge;

use crate::config::widgets::wrapbox::BoxConfig;
use crate::ui::draws::frame_manager::{FrameManager, FrameManagerBindTransition};
use crate::ui::draws::transition_state::{self, TransitionStateList, TransitionStateRc};
use crate::ui::draws::util::Z;

use super::expose::BoxExposeRc;
use super::BoxCtxRc;

pub struct DrawCore {
    box_ctx: BoxCtxRc,

    box_frame_manager: FrameManager,
    ts_list: TransitionStateList,
    pub box_motion_transition: TransitionStateRc,

    // func
    motion_func: DrawMotion,
    input_size_func: SetWindowInputSize,
}
impl DrawCore {
    pub fn new(
        darea: &DrawingArea,

        box_conf: &mut BoxConfig,
        box_ctx: BoxCtxRc,
        expose: &BoxExposeRc,
        edge: Edge,
        position: Edge,
        extra_trigger_size: f64,
    ) -> Self {
        let box_frame_manager = {
            let up = expose.borrow().update_func();
            FrameManager::new(
                box_conf.box_conf.frame_rate,
                glib::clone!(move || {
                    up();
                }),
            )
        };
        let mut ts_list = TransitionStateList::new();
        let pop_ts = ts_list
            .new_transition(Duration::from_millis(box_conf.box_conf.transition_duration))
            .item;

        let motion_func = make_motion_func(edge, position);
        let input_size_func = make_window_input_size_func(edge, position, extra_trigger_size);

        {
            let mut box_ctx = box_ctx.borrow_mut();
            let content = box_ctx.grid_box.draw_content();
            let content = {
                let content_size = (content.width(), content.height());
                box_ctx.outlook.redraw_if_size_change(content_size);
                box_ctx.outlook.with_box(content)
            };

            let wh = (content.width(), content.height());
            set_window_max_size(darea, wh);
        }

        Self {
            box_ctx,

            box_frame_manager,
            ts_list,
            box_motion_transition: pop_ts,

            motion_func,
            input_size_func,
        }
    }

    pub fn draw(
        &mut self,
        ctx: &cairo::Context,
        darea: &DrawingArea,
        window: &gtk::ApplicationWindow,
    ) {
        let (content, y) = {
            let mut box_ctx = self.box_ctx.borrow_mut();
            self.ts_list.refresh();

            let content = box_ctx.grid_box.draw_content();
            let content = {
                let content_size = (content.width(), content.height());
                box_ctx.outlook.redraw_if_size_change(content_size);
                box_ctx.outlook.with_box(content)
            };

            let wh = (content.width(), content.height());
            let y = self.box_motion_transition.borrow().get_y();

            set_window_max_size(darea, wh);
            let input_region = (self.input_size_func)(window, darea, wh, y);
            self.box_frame_manager.ensure_frame_run(&self.ts_list);

            box_ctx.update_input_region(input_region);

            (content, y)
        };

        let size = (content.width() as f64, content.height() as f64);

        (self.motion_func)(ctx, darea, size, y);

        ctx.set_source_surface(&content, Z, Z).unwrap();
        ctx.paint().unwrap()
    }
}

fn set_window_max_size(darea: &DrawingArea, size: (i32, i32)) {
    darea.set_size_request(size.0, size.1);
}

type DrawMotion = Box<dyn Fn(&cairo::Context, &DrawingArea, (f64, f64), f64)>;
fn make_motion_func(edge: Edge, position: Edge) -> DrawMotion {
    // NOTE: WE NEED BETTER CODE FOR THIS.

    macro_rules! match_x {
        // position left
        ($i:ident; P; L) => {
            let $i = Z;
        };
        // position middle
        ($i:ident, $darea:expr, $size:expr; P; M) => {
            let $i = (calculate_x_additional($darea, $size) / 2.).floor();
        };
        // position right
        ($i:ident, $darea:expr, $size:expr; P; R) => {
            let $i = calculate_x_additional($darea, $size).floor();
        };
        // edge left
        ($i:ident, $size:expr, $visible_y:expr; E; L) => {
            let $i = (-$size.0 + $visible_y).floor();
        };
        // edge right
        ($i:ident, $darea:expr, $size:expr, $visible_y:expr; E; R) => {
            let a = calculate_x_additional($darea, $size).ceil();
            let $i = ($size.0 - $visible_y).ceil() + a;
        };
    }
    macro_rules! match_y {
        // position top
        ($i:ident; P; T) => {
            let $i = Z;
        };
        // position middle
        ($i:ident, $darea:expr, $size:expr; P; M) => {
            let $i = (calculate_y_additional($darea, $size) / 2.).floor();
        };
        // position bottom
        ($i:ident, $darea:expr, $size:expr; P; B) => {
            let $i = calculate_y_additional($darea, $size).floor();
        };
        // edge top
        ($i:ident, $size:expr, $visible_y:expr; E; T) => {
            let $i = (-$size.1 + $visible_y).floor();
        };
        // edge bottom
        ($i:ident, $darea:expr, $size:expr, $visible_y:expr; E; B) => {
            let a = calculate_y_additional($darea, $size).ceil();
            let $i = ($size.1 - $visible_y).ceil() + a;
        };
    }

    type DrawMotionFunc = Box<dyn Fn(&cairo::Context, &DrawingArea, (f64, f64), f64)>;

    // 12 different cases
    let draw_motion: DrawMotionFunc = Box::new(match (edge, position) {
        // left center
        (Edge::Left, Edge::Left) | (Edge::Left, Edge::Right) => |ctx, darea, size, visible_y| {
            match_x!(x, size, visible_y; E; L);
            match_y!(y, darea, size; P; M);
            ctx.translate(x, y)
        },
        // left top
        (Edge::Left, Edge::Top) => |ctx, _, size, visible_y| {
            match_x!(x, size, visible_y; E; L);
            match_y!(y; P; T);
            ctx.translate(x, y)
        },
        // left bottom
        (Edge::Left, Edge::Bottom) => |ctx, darea, size, visible_y| {
            match_x!(x, size, visible_y; E; L);
            match_y!(y, darea, size; P; B);
            ctx.translate(x, y)
        },
        // right center
        (Edge::Right, Edge::Left) | (Edge::Right, Edge::Right) => |ctx, darea, size, visible_y| {
            match_x!(x, darea, size, visible_y; E; R);
            match_y!(y, darea, size; P; M);
            ctx.translate(x, y)
        },
        // right top
        (Edge::Right, Edge::Top) => |ctx, darea, size, visible_y| {
            match_x!(x, darea, size, visible_y; E; R);
            match_y!(y; P; T);
            ctx.translate(x, y)
        },
        // right bottom
        (Edge::Right, Edge::Bottom) => |ctx, darea, size, visible_y| {
            match_x!(x, darea, size, visible_y; E; R);
            match_y!(y, darea, size; P; B);
            ctx.translate(x, y)
        },
        // top center
        (Edge::Top, Edge::Top) | (Edge::Top, Edge::Bottom) => |ctx, darea, size, visible_y| {
            match_x!(x, darea, size; P; M);
            match_y!(y, size, visible_y; E; T);
            ctx.translate(x, y)
        },
        // top left
        (Edge::Top, Edge::Left) => |ctx, _, size, visible_y| {
            match_x!(x; P; L);
            match_y!(y, size, visible_y; E; T);
            ctx.translate(x, y)
        },
        // top right
        (Edge::Top, Edge::Right) => |ctx, darea, size, visible_y| {
            match_x!(x, darea, size; P; R);
            match_y!(y, size, visible_y; E; T);
            ctx.translate(x, y)
        },
        // bottom center
        (Edge::Bottom, Edge::Top) | (Edge::Bottom, Edge::Bottom) => {
            |ctx, darea, size, visible_y| {
                match_x!(x, darea, size; P; M);
                match_y!(y, darea, size, visible_y; E; B);
                ctx.translate(x, y)
            }
        }
        // bottom left
        (Edge::Bottom, Edge::Left) => |ctx, darea, size, visible_y| {
            match_x!(x; P; L);
            match_y!(y, darea, size, visible_y; E; B);
            ctx.translate(x, y)
        },
        // bottom right
        (Edge::Bottom, Edge::Right) => |ctx, darea, size, visible_y| {
            match_x!(x, darea, size; P; R);
            match_y!(y, darea, size, visible_y; E; B);
            ctx.translate(x, y)
        },
        _ => unreachable!(),
    });

    let get_range: Box<dyn Fn((f64, f64)) -> f64> = Box::new(match edge {
        Edge::Left | Edge::Right => |size| size.0,
        Edge::Top | Edge::Bottom => |size| size.1,
        _ => unreachable!(),
    });

    Box::new(move |ctx, darea, size, y| {
        let range = get_range(size);
        let visible_y = transition_state::calculate_transition(y, (Z, range));
        draw_motion(ctx, darea, size, visible_y);
    })
}

type SetWindowInputSize =
    Box<dyn Fn(&gtk::ApplicationWindow, &DrawingArea, (i32, i32), f64) -> RectangleInt>;
fn make_window_input_size_func(
    edge: Edge,
    position: Edge,
    extra_trigger_size: f64,
) -> SetWindowInputSize {
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
        ($i:ident; P; L) => {
            let $i = 0;
        };
        // position middle
        ($i:ident, $darea:expr, $size:expr; P; M) => {
            let $i = (calculate_x_additional($darea, ($size.0 as f64, $size.1 as f64)) / 2.).ceil()
                as i32;
        };
        // position right
        ($i:ident, $darea:expr, $size:expr; P; R) => {
            let $i = calculate_x_additional($darea, ($size.0 as f64, $size.1 as f64)).ceil() as i32;
        };
        // edge left
        ($i:ident; E; L) => {
            let $i = 0;
        };
        // edge right
        ($i:ident, $darea:expr, $size:expr, $ts_y:expr; E; R) => {
            let $i = (($size.0 as f64) * (1. - $ts_y)) as i32
                + calculate_x_additional($darea, ($size.0 as f64, $size.1 as f64)).floor() as i32;
        };
    }
    macro_rules! match_y {
        // position top
        ($i:ident; P; T) => {
            let $i = 0;
        };
        // position middle
        ($i:ident, $darea:expr, $size:expr; P; M) => {
            let $i = (calculate_y_additional($darea, ($size.0 as f64, $size.1 as f64)) / 2.).ceil()
                as i32;
        };
        // position bottom
        ($i:ident, $darea:expr, $size:expr; P; B) => {
            let $i = calculate_y_additional($darea, ($size.0 as f64, $size.1 as f64)).ceil() as i32;
        };
        // edge top
        ($i:ident; E; T) => {
            let $i = 0;
        };
        // edge bottom
        ($i:ident, $darea:expr, $size:expr, $ts_y:expr; E; B) => {
            let $i = (($size.1 as f64) * (1. - $ts_y)) as i32
                + calculate_y_additional($darea, ($size.0 as f64, $size.1 as f64)).floor() as i32;
        };
    }
    type GetXYWH = Box<dyn Fn(&DrawingArea, (i32, i32), f64) -> [i32; 4]>;
    let get_xywh: GetXYWH = Box::new(match (edge, position) {
        (Edge::Left, Edge::Right) | (Edge::Left, Edge::Left) => |darea, size, ts_y| {
            match_x!(x; E; L);
            match_y!(y, darea, size; P; M);
            edge_wh!(w, h, size, ts_y; H);
            [x, y, w, h]
        },
        (Edge::Left, Edge::Top) => |_, size, ts_y| {
            match_x!(x; E; L);
            match_y!(y; P; T);
            edge_wh!(w, h, size, ts_y; H);
            [x, y, w, h]
        },
        (Edge::Left, Edge::Bottom) => |darea, size, ts_y| {
            match_x!(x; E; L);
            match_y!(y, darea, size; P; B);
            edge_wh!(w, h, size, ts_y; H);
            [x, y, w, h]
        },
        (Edge::Right, Edge::Right) | (Edge::Right, Edge::Left) => |darea, size, ts_y| {
            match_x!(x, darea, size, ts_y; E; R);
            match_y!(y, darea, size; P; M);
            edge_wh!(w, h, size, ts_y; H);
            [x, y, w, h]
        },
        (Edge::Right, Edge::Top) => |darea, size, ts_y| {
            match_x!(x, darea, size, ts_y; E; R);
            match_y!(y; P; T);
            edge_wh!(w, h, size, ts_y; H);
            [x, y, w, h]
        },
        (Edge::Right, Edge::Bottom) => |darea, size, ts_y| {
            match_x!(x, darea, size, ts_y; E; R);
            match_y!(y, darea, size; P; B);
            edge_wh!(w, h, size, ts_y; H);
            [x, y, w, h]
        },
        (Edge::Top, Edge::Left) => |_, size, ts_y| {
            match_x!(x; P; L);
            match_y!(y; E; T);
            edge_wh!(w, h, size, ts_y; V);
            [x, y, w, h]
        },
        (Edge::Top, Edge::Right) => |darea, size, ts_y| {
            match_x!(x, darea, size; P; R);
            match_y!(y; E; T);
            edge_wh!(w, h, size, ts_y; V);
            [x, y, w, h]
        },
        (Edge::Top, Edge::Top) | (Edge::Top, Edge::Bottom) => |darea, size, ts_y| {
            match_x!(x, darea, size; P; M);
            match_y!(y; E; T);
            edge_wh!(w, h, size, ts_y; V);
            [x, y, w, h]
        },
        (Edge::Bottom, Edge::Left) => |darea, size, ts_y| {
            match_x!(x; P; L);
            match_y!(y, darea, size, ts_y; E; B);
            edge_wh!(w, h, size, ts_y; V);
            [x, y, w, h]
        },
        (Edge::Bottom, Edge::Right) => |darea, size, ts_y| {
            match_x!(x, darea, size; P; R);
            match_y!(y, darea, size, ts_y; E; B);
            edge_wh!(w, h, size, ts_y; V);
            [x, y, w, h]
        },
        (Edge::Bottom, Edge::Top) | (Edge::Bottom, Edge::Bottom) => |darea, size, ts_y| {
            match_x!(x, darea, size; P; M);
            match_y!(y, darea, size, ts_y; E; B);
            edge_wh!(w, h, size, ts_y; V);
            [x, y, w, h]
        },
        _ => unreachable!(),
    });

    type GetInr = Box<dyn Fn(&mut RectangleInt, f64)>;
    let get_inr: GetInr = Box::new(match edge {
        Edge::Top => |inr, extra_trigger_size| {
            inr.set_height(inr.height() + extra_trigger_size as i32);
        },
        Edge::Bottom => |inr, extra_trigger_size| {
            inr.set_y(inr.y() - extra_trigger_size as i32);
            inr.set_height(inr.height() + extra_trigger_size as i32);
        },
        Edge::Left => |inr, extra_trigger_size| {
            inr.set_width(inr.width() + extra_trigger_size as i32);
        },
        Edge::Right => |inr, extra_trigger_size| {
            inr.set_x(inr.x() - extra_trigger_size as i32);
            inr.set_width(inr.width() + extra_trigger_size as i32);
        },
        _ => unreachable!(),
    });

    Box::new(move |window, darea, size, ts_y| {
        let [x, y, w, h] = get_xywh(darea, size, ts_y);
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

fn calculate_x_additional(darea: &DrawingArea, size: (f64, f64)) -> f64 {
    (darea.width() as f64).max(size.0) - size.0
}
fn calculate_y_additional(darea: &DrawingArea, size: (f64, f64)) -> f64 {
    (darea.height() as f64).max(size.1) - size.1
}
