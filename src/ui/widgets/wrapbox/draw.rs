use std::time::Duration;
use std::{cell::RefCell, rc::Rc};

use async_channel::Receiver;
use cairo::{ImageSurface, RectangleInt, Region};
use gio::glib::clone::Downgrade;
use gtk::glib;
use gtk::prelude::NativeExt;
use gtk::prelude::{DrawingAreaExtManual, GtkWindowExt, SurfaceExt, WidgetExt};
use gtk::DrawingArea;
use gtk4_layer_shell::Edge;

use crate::config::widgets::wrapbox::{BoxConfig, BoxedWidgetConfig};
use crate::config::Config;
use crate::ui::draws::frame_manager::{FrameManager, FrameManagerBindTransition};
use crate::ui::draws::transition_state::{self, TransitionStateList, TransitionStateRc};
use crate::ui::draws::util::{draw_motion, draw_rotation, new_surface, Z};
use crate::ui::WidgetExposePtr;

use super::expose::BoxExposeRc;
use super::{BoxBuffer, BoxCtxRc};

type DrawMotion = Box<dyn Fn(&cairo::Context, &DrawingArea, (f64, f64), f64)>;

pub struct DrawCore {
    box_ctx: BoxCtxRc,

    // config
    edge: Edge,
    position: Edge,
    extra_trigger_size: f64,

    box_frame_manager: FrameManager,
    ts_list: TransitionStateList,
    pub box_motion_transition: TransitionStateRc,
}
impl DrawCore {
    pub fn new(
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

        Self {
            box_ctx,

            edge,
            position,
            extra_trigger_size,

            box_frame_manager,
            ts_list,
            box_motion_transition: pop_ts,
        }
    }

    pub fn refresh(&mut self, darea: &DrawingArea, window: &gtk::ApplicationWindow) -> BoxBuffer {
        let mut box_ctx = self.box_ctx.borrow_mut();
        self.ts_list.refresh();

        let (content, filtered_map) = box_ctx.grid_box.draw_content();
        // let content = rotate_content(self.edge, content);
        let content = {
            let content_size = (content.width(), content.height());
            box_ctx.outlook.redraw_if_size_change(content_size);
            box_ctx.outlook.with_box(content)
        };

        let wh = (content.width(), content.height());

        let y = self.box_motion_transition.borrow().get_y();

        set_window_max_size(darea, wh);
        let rec_int = set_window_input_size(
            window,
            darea,
            self.edge,
            self.position,
            wh,
            y,
            self.extra_trigger_size,
        );

        let buffer = {
            self.box_frame_manager.ensure_frame_run(&self.ts_list);
            BoxBuffer { content, y }
        };

        box_ctx.r(filtered_map, rec_int);
        buffer
    }

    fn core(&self) {
        let edge = self.edge;
        let get_visible_y = Box::new(move |y: f64, size: (f64, f64)| {
            let range = match edge {
                Edge::Left | Edge::Right => size.0,
                Edge::Top | Edge::Bottom => size.1,
                _ => unreachable!(),
            };
            transition_state::calculate_transition(y, (Z, range))
        });

        let draw_motion: DrawMotion = Box::new(match self.edge {
            Edge::Left => |ctx, _, size, visible_y| ctx.translate((-size.0 + visible_y).floor(), Z),
            Edge::Right => |ctx, darea, size, visible_y| {
                ctx.translate(
                    (size.0 - visible_y).ceil() + (darea.width() as f64 - size.0).ceil(),
                    Z,
                )
            },
            Edge::Top | Edge::Bottom => match self.position {
                Edge::Right => |ctx, darea, size, visible_y| {
                    let x = (darea.width() as f64 - size.0).floor();
                    ctx.translate(x, (-size.1 + visible_y).floor())
                },
                Edge::Top | Edge::Bottom => |ctx, darea, size, visible_y| {
                    let x = ((darea.width() as f64 - size.0) / 2.).floor();
                    ctx.translate(x, (-size.1 + visible_y).floor())
                },
                _ => |ctx, _, size, visible_y| ctx.translate(Z, (-size.1 + visible_y).floor()),
            },
            // Edge::Bottom => match self.position {
            //     Edge::Right => |ctx, darea, size, visible_y| {
            //         let x = (darea.width() as f64 - size.0).floor();
            //         ctx.translate(x, (size.1 - visible_y).ceil())
            //     },
            //     Edge::Top | Edge::Bottom => |ctx, darea, size, visible_y| {
            //         let x = ((darea.width() as f64 - size.0) / 2.).floor();
            //         ctx.translate(x, (-size.1 + visible_y).floor())
            //     },
            //     Edge::Right => |ctx, _, size, visible_y| {
            //         ctx.translate(Z, (-size.1 + visible_y).floor())
            //     },
            // },
            // Edge::Bottom => {
            //     let x = match self.position {
            //         Edge::Right => (darea.width() as f64 - size.0).floor(),
            //         Edge::Top | Edge::Bottom => {
            //             ((darea.width() as f64 - size.0) / 2.).floor()
            //         }
            //         _ => Z,
            //     };
            //     ctx.translate(x, (size.1 - visible_y).ceil())
            // }
            _ => unreachable!(),
        });
    }

    pub fn draw(
        &mut self,
        ctx: &cairo::Context,
        darea: &DrawingArea,
        window: &gtk::ApplicationWindow,
    ) {
        let buf = self.refresh(darea, window);
        let size = (buf.content.width() as f64, buf.content.height() as f64);

        let range = match self.edge {
            Edge::Left | Edge::Right => size.0,
            Edge::Top | Edge::Bottom => size.1,
            _ => todo!(),
        };
        let visible_y = transition_state::calculate_transition(buf.y, (Z, range));

        self.motion(ctx, darea, size, visible_y);

        ctx.set_source_surface(&buf.content, Z, Z).unwrap();
        ctx.paint().unwrap()
    }

    fn motion(&self, ctx: &cairo::Context, darea: &DrawingArea, size: (f64, f64), visible_y: f64) {
        match self.edge {
            Edge::Left => ctx.translate((-size.0 + visible_y).floor(), Z),
            Edge::Right => ctx.translate(
                (size.0 - visible_y).ceil() + (darea.width() as f64 - size.0).ceil(),
                Z,
            ),
            Edge::Top => {
                let x = match self.position {
                    Edge::Right => (darea.width() as f64 - size.0).floor(),
                    Edge::Top | Edge::Bottom => ((darea.width() as f64 - size.0) / 2.).floor(),
                    _ => Z,
                };
                ctx.translate(x, (-size.1 + visible_y).floor())
            }
            Edge::Bottom => {
                let x = match self.position {
                    Edge::Right => (darea.width() as f64 - size.0).floor(),
                    Edge::Top | Edge::Bottom => ((darea.width() as f64 - size.0) / 2.).floor(),
                    _ => Z,
                };
                ctx.translate(x, (size.1 - visible_y).ceil())
            }
            _ => unreachable!(),
        }
    }
}

fn set_window_max_size(darea: &DrawingArea, size: (i32, i32)) {
    darea.set_size_request(size.0, size.1);
}

fn set_window_input_size(
    window: &gtk::ApplicationWindow,
    darea: &DrawingArea,
    edge: Edge,
    position: Edge,
    size: (i32, i32),
    ts_y: f64,
    extra_trigger_size: f64,
) -> RectangleInt {
    let x = match (edge, position) {
        (Edge::Left, Edge::Left)
        | (Edge::Left, Edge::Right)
        | (Edge::Left, Edge::Top)
        | (Edge::Left, Edge::Bottom)
        | (Edge::Top, Edge::Left)
        | (Edge::Bottom, Edge::Left) => 0,
        (Edge::Right, Edge::Left)
        | (Edge::Right, Edge::Right)
        | (Edge::Right, Edge::Top)
        | (Edge::Right, Edge::Bottom) => {
            ((size.0 as f64) * (1. - ts_y)) as i32 + (darea.width().max(size.0) - size.0)
        }
        (Edge::Top, Edge::Right) | (Edge::Bottom, Edge::Right) => {
            darea.width().max(size.0) - size.0
        }
        (Edge::Top, Edge::Top)
        | (Edge::Top, Edge::Bottom)
        | (Edge::Bottom, Edge::Top)
        | (Edge::Bottom, Edge::Bottom) => (darea.width().max(size.0) - size.0) / 2,
        _ => unreachable!(),
    };
    let (y, h) = match edge {
        Edge::Top => (Z as i32, (size.1 as f64 * ts_y) as i32),
        Edge::Bottom => (
            (size.1 as f64 * (1. - ts_y)) as i32,
            (size.1 as f64 * ts_y).ceil() as i32,
        ),
        _ => (Z as i32, size.1),
    };
    let w = match edge {
        Edge::Left | Edge::Right => (size.0 as f64 * ts_y).ceil() as i32,
        Edge::Top | Edge::Bottom => size.0,
        _ => unreachable!(),
    };

    // box normal input region
    let rec_int = RectangleInt::new(x, y, w, h);

    // box input region add extra_trigger
    let mut inr = rec_int;
    match edge {
        Edge::Top => {
            inr.set_height(inr.height() + extra_trigger_size as i32);
        }
        Edge::Bottom => {
            inr.set_y(inr.y() - extra_trigger_size as i32);
            inr.set_height(inr.height() + extra_trigger_size as i32);
        }
        Edge::Left => {
            inr.set_width(inr.width() + extra_trigger_size as i32);
        }
        Edge::Right => {
            inr.set_x(inr.x() - extra_trigger_size as i32);
            inr.set_width(inr.width() + extra_trigger_size as i32);
        }
        _ => {}
    }

    if let Some(surf) = window.surface() {
        surf.set_input_region(&Region::create_rectangle(&inr));
    }

    rec_int
}
