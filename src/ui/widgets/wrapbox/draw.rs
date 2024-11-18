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

type DrawMotion = Box<dyn Fn(&cairo::Context, &DrawingArea, (f64, f64), f64)>;

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

        let motion_func = Self::make_motion_func(edge, position);
        let input_size_func = set_window_input_size_func(edge, position, extra_trigger_size);

        {
            let mut box_ctx = box_ctx.borrow_mut();
            let (content, _) = box_ctx.grid_box.draw_content();
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

            let (content, filtered_map) = box_ctx.grid_box.draw_content();
            let content = {
                let content_size = (content.width(), content.height());
                box_ctx.outlook.redraw_if_size_change(content_size);
                box_ctx.outlook.with_box(content)
            };

            let wh = (content.width(), content.height());
            let y = self.box_motion_transition.borrow().get_y();

            set_window_max_size(darea, wh);
            let rec_int = (self.input_size_func)(window, darea, wh, y);
            self.box_frame_manager.ensure_frame_run(&self.ts_list);

            box_ctx.update_box_ctx(filtered_map, rec_int);

            (content, y)
        };

        let size = (content.width() as f64, content.height() as f64);

        (self.motion_func)(ctx, darea, size, y);

        ctx.set_source_surface(&content, Z, Z).unwrap();
        ctx.paint().unwrap()
    }

    fn make_motion_func(edge: Edge, position: Edge) -> DrawMotion {
        type DrawMotionFunc = Box<dyn Fn(&cairo::Context, &DrawingArea, (f64, f64), f64)>;
        let draw_motion: DrawMotionFunc = Box::new(match edge {
            Edge::Left => match position {
                Edge::Left | Edge::Right => |ctx, darea, size, visible_y| {
                    let y = (calculate_y_additional(darea, size) / 2.).floor();
                    ctx.translate((-size.0 + visible_y).floor(), y)
                },
                Edge::Top => {
                    |ctx, _, size, visible_y| ctx.translate((-size.0 + visible_y).floor(), Z)
                }
                Edge::Bottom => |ctx, darea, size, visible_y| {
                    let y = calculate_y_additional(darea, size).floor();
                    ctx.translate((-size.0 + visible_y).floor(), y)
                },
                _ => unreachable!(),
            },
            Edge::Right => match position {
                Edge::Left | Edge::Right => |ctx, darea, size, visible_y| {
                    let x = calculate_x_additional(darea, size).ceil();
                    let y = (calculate_y_additional(darea, size) / 2.).floor();
                    ctx.translate((size.0 - visible_y).ceil() + x, y)
                },
                Edge::Top => |ctx, darea, size, visible_y| {
                    let x = calculate_x_additional(darea, size).ceil();
                    ctx.translate((size.0 - visible_y).ceil() + x, Z)
                },
                Edge::Bottom => |ctx, darea, size, visible_y| {
                    let x = calculate_x_additional(darea, size).ceil();
                    let y = (calculate_y_additional(darea, size)).floor();
                    ctx.translate((size.0 - visible_y).ceil() + x, y)
                },
                _ => unreachable!(),
            },
            Edge::Top => match position {
                Edge::Right => |ctx, darea, size, visible_y| {
                    let x = calculate_x_additional(darea, size).floor();
                    ctx.translate(x, (-size.1 + visible_y).floor())
                },
                Edge::Top | Edge::Bottom => |ctx, darea, size, visible_y| {
                    let x = (calculate_x_additional(darea, size) / 2.).floor();
                    ctx.translate(x, (-size.1 + visible_y).floor())
                },
                _ => |ctx, _, size, visible_y| ctx.translate(Z, (-size.1 + visible_y).floor()),
            },
            Edge::Bottom => match position {
                Edge::Right => |ctx, darea, size, visible_y| {
                    let x = calculate_x_additional(darea, size).floor();
                    let y = calculate_y_additional(darea, size).floor();
                    ctx.translate(x, (size.1 - visible_y + y).ceil())
                },
                Edge::Top | Edge::Bottom => |ctx, darea, size, visible_y| {
                    let x = (calculate_x_additional(darea, size) / 2.).floor();
                    let y = calculate_y_additional(darea, size).floor();
                    ctx.translate(x, (size.1 - visible_y + y).ceil())
                },
                _ => |ctx, darea, size, visible_y| {
                    let y = calculate_y_additional(darea, size).floor();
                    ctx.translate(Z, (size.1 - visible_y + y).ceil())
                },
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
}

fn set_window_max_size(darea: &DrawingArea, size: (i32, i32)) {
    darea.set_size_request(size.0, size.1);
}

type SetWindowInputSize =
    Box<dyn Fn(&gtk::ApplicationWindow, &DrawingArea, (i32, i32), f64) -> RectangleInt>;
fn set_window_input_size_func(
    edge: Edge,
    position: Edge,
    extra_trigger_size: f64,
) -> SetWindowInputSize {
    type GetX = Box<dyn Fn(&DrawingArea, (i32, i32), f64) -> i32>;

    let get_x: GetX = Box::new(match (edge, position) {
        (Edge::Left, Edge::Left)
        | (Edge::Left, Edge::Right)
        | (Edge::Left, Edge::Top)
        | (Edge::Left, Edge::Bottom)
        | (Edge::Top, Edge::Left)
        | (Edge::Bottom, Edge::Left) => |_, _, _| 0,
        (Edge::Right, Edge::Left)
        | (Edge::Right, Edge::Right)
        | (Edge::Right, Edge::Top)
        | (Edge::Right, Edge::Bottom) => |darea, size, ts_y| {
            ((size.0 as f64) * (1. - ts_y)) as i32
                + calculate_x_additional(darea, (size.0 as f64, size.1 as f64)).floor() as i32
        },
        (Edge::Top, Edge::Right) | (Edge::Bottom, Edge::Right) => |darea, size, _| {
            calculate_x_additional(darea, (size.0 as f64, size.1 as f64)).floor() as i32
        },
        (Edge::Top, Edge::Top)
        | (Edge::Top, Edge::Bottom)
        | (Edge::Bottom, Edge::Top)
        | (Edge::Bottom, Edge::Bottom) => |darea, size, _| {
            (calculate_x_additional(darea, (size.0 as f64, size.1 as f64)) / 2.).floor() as i32
        },
        _ => unreachable!(),
    });

    type GetY = Box<dyn Fn(&DrawingArea, (i32, i32), f64) -> i32>;
    let get_y: GetY = Box::new(match (edge, position) {
        (Edge::Right, Edge::Left)
        | (Edge::Right, Edge::Right)
        | (Edge::Left, Edge::Left)
        | (Edge::Left, Edge::Right) => |darea, size, _| {
            (calculate_y_additional(darea, (size.0 as f64, size.1 as f64)) / 2.).ceil() as i32
        },
        (Edge::Top, Edge::Left)
        | (Edge::Top, Edge::Right)
        | (Edge::Top, Edge::Top)
        | (Edge::Top, Edge::Bottom)
        | (Edge::Right, Edge::Top)
        | (Edge::Left, Edge::Top) => |_, _, _| 0,
        (Edge::Right, Edge::Bottom) | (Edge::Left, Edge::Bottom) => |darea, size, _| {
            calculate_y_additional(darea, (size.0 as f64, size.1 as f64)).floor() as i32
        },
        (Edge::Bottom, Edge::Left)
        | (Edge::Bottom, Edge::Right)
        | (Edge::Bottom, Edge::Top)
        | (Edge::Bottom, Edge::Bottom) => |darea, size, ts_y| {
            ((size.1 as f64) * (1. - ts_y)) as i32
                + calculate_y_additional(darea, (size.0 as f64, size.1 as f64)).floor() as i32
        },
        _ => unreachable!(),
    });

    type GetW = Box<dyn Fn((i32, i32), f64) -> i32>;
    let get_w: GetW = Box::new(match edge {
        Edge::Left | Edge::Right => |size, ts_y| (size.0 as f64 * ts_y).ceil() as i32,
        Edge::Top | Edge::Bottom => |size, _| size.0,
        _ => unreachable!(),
    });

    type GetH = Box<dyn Fn((i32, i32), f64) -> i32>;
    let get_h: GetH = Box::new(match edge {
        Edge::Bottom | Edge::Top => |size, ts_y| (size.1 as f64 * ts_y) as i32,
        Edge::Left | Edge::Right => |size, _| size.1,
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
        let x = get_x(darea, size, ts_y);
        let y = get_y(darea, size, ts_y);
        let w = get_w(size, ts_y);
        let h = get_h(size, ts_y);

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
