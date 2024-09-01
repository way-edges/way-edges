use std::time::Duration;
use std::{cell::RefCell, rc::Rc};

use async_channel::Receiver;
use cairo::{ImageSurface, RectangleInt, Region};
use display::grid::{GridBox, GridItemSizeMap};
use event::event_handle;
use expose::{BoxExpose, BoxExposeRc, BoxWidgetExpose};
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

use super::ring::init_ring;
use super::text::init_text;

pub mod display;
mod event;
pub mod expose;
pub mod outlook;

pub type MousePosition = (f64, f64);

struct BoxBuffer {
    content: ImageSurface,
    y: f64,
}

type BoxCtxRc = Rc<RefCell<BoxCtx>>;

struct BoxCtx {
    // use
    buf: BoxBuffer,
    item_map: GridItemSizeMap,
    rec_int: RectangleInt,
    outlook: outlook::window::BoxOutlookWindow,
    grid_box: GridBox,

    // config
    edge: Edge,
    position: Edge,
    extra_trigger_size: f64,

    box_frame_manager: FrameManager,
    ts_list: TransitionStateList,
    box_motion_transition: TransitionStateRc,
}

impl BoxCtx {
    fn new(
        window: &gtk::ApplicationWindow,
        darea: &DrawingArea,
        box_conf: &mut BoxConfig,
        edge: Edge,
        position: Edge,
        extra_trigger_size: f64,
    ) -> (Self, Receiver<()>, BoxExposeRc, TransitionStateRc) {
        let mut grid_box =
            display::grid::GridBox::new(box_conf.box_conf.gap, box_conf.box_conf.align);

        // define box expose and create boxed widgets
        let (expose, update_signal_receiver) = BoxExpose::new();
        init_boxed_widgets(
            &mut grid_box,
            expose.clone(),
            std::mem::take(&mut box_conf.widgets),
        );

        // draw first frame
        // first draw
        let (content, item_map) = grid_box.draw_content();
        let content = rotate_content(edge, content);

        // create outlook
        let ol = match box_conf.outlook.take().unwrap() {
            crate::config::widgets::wrapbox::Outlook::Window(c) => {
                outlook::window::BoxOutlookWindow::new(c, (content.width(), content.height()))
            }
        };

        // add box outlook
        let content = ol.with_box(content);
        let content_size = set_window_max_size(darea, (content.width(), content.height()), edge);

        // input region
        let rec_int = set_window_input_size(
            window,
            darea,
            edge,
            position,
            content_size,
            Z,
            extra_trigger_size,
        );

        let buf = BoxBuffer { content, y: Z };

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
        (
            Self {
                buf,
                item_map,
                rec_int,
                outlook: ol,
                grid_box,

                edge,
                position,
                extra_trigger_size,

                box_frame_manager,
                ts_list,
                box_motion_transition: pop_ts.clone(),
            },
            update_signal_receiver,
            expose,
            pop_ts,
        )
    }

    fn refresh(&mut self, darea: &DrawingArea, window: &gtk::ApplicationWindow) {
        self.ts_list.refresh();

        let (content, filtered_map) = self.grid_box.draw_content();
        let content = rotate_content(self.edge, content);
        let content = {
            let content_size = (content.width(), content.height());
            self.outlook.redraw_if_size_change(content_size);
            self.outlook.with_box(content)
        };

        let wh = set_window_max_size(darea, (content.width(), content.height()), self.edge);

        let y = self.box_motion_transition.borrow().get_y();

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

        self.buf = buffer;
        self.item_map = filtered_map;
        self.rec_int = rec_int;
    }
}

impl Drop for BoxCtx {
    fn drop(&mut self) {
        log::info!("drop box ctx");
    }
}

fn set_window_max_size(darea: &DrawingArea, size: (i32, i32), edge: Edge) -> (i32, i32) {
    let size = match edge {
        Edge::Left | Edge::Right => size,
        Edge::Top | Edge::Bottom => (size.1, size.0),
        _ => unreachable!(),
    };
    darea.set_size_request(size.0, size.1);
    size
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
        | (Edge::Bottom, Edge::Left) => Z as i32,
        (Edge::Right, Edge::Left)
        | (Edge::Right, Edge::Right)
        | (Edge::Right, Edge::Top)
        | (Edge::Right, Edge::Bottom)
        | (Edge::Top, Edge::Right)
        | (Edge::Bottom, Edge::Right) => ((size.0 as f64) * (1. - ts_y)) as i32,
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

pub fn init_widget(
    window: &gtk::ApplicationWindow,
    conf: Config,
    mut box_conf: BoxConfig,
) -> Result<WidgetExposePtr, String> {
    let edge = conf.edge;
    let position = conf.position.unwrap();
    let extra_trigger_size = box_conf.box_conf.extra_trigger_size.get_num_into().unwrap();

    let darea = DrawingArea::new();
    window.set_child(Some(&darea));
    darea.connect_destroy(|_| {
        log::info!("destroy `box` drawing area");
    });

    let (box_ctx, update_signal_receiver, expose, box_motion_transition) = BoxCtx::new(
        window,
        &darea,
        &mut box_conf,
        edge,
        position,
        extra_trigger_size,
    );
    let box_ctx = Rc::new(RefCell::new(box_ctx));

    // it's a async block once, doesn't matter strong or weak
    glib::spawn_future_local(glib::clone!(
        #[weak]
        darea,
        async move {
            log::debug!("box draw signal receive loop start");
            while (update_signal_receiver.recv().await).is_ok() {
                darea.queue_draw();
            }
            log::debug!("box draw signal receive loop exit");
        }
    ));

    darea.set_draw_func(glib::clone!(
        #[strong]
        box_ctx,
        #[weak]
        window,
        move |darea, ctx, _, _| {
            box_ctx.borrow_mut().refresh(darea, &window);

            let buf = &box_ctx.borrow().buf;

            let size = (buf.content.width() as f64, buf.content.height() as f64);
            let range = (0., size.0);
            let visible_y = transition_state::calculate_transition(buf.y, range);
            draw_rotation(ctx, edge, size);
            match edge {
                Edge::Top => match position {
                    Edge::Right => {
                        ctx.translate(0., -(darea.width() as f64 - size.1));
                    }
                    Edge::Top | Edge::Bottom => {
                        ctx.translate(0., -(darea.width() as f64 - size.1) / 2.);
                    }
                    _ => {}
                },
                Edge::Bottom => match position {
                    Edge::Right => {
                        ctx.translate(0., darea.width() as f64 - size.1);
                    }
                    Edge::Top | Edge::Bottom => {
                        ctx.translate(0., (darea.width() as f64 - size.1) / 2.);
                    }
                    _ => {}
                },
                _ => {}
            };
            draw_motion(ctx, visible_y, range);

            ctx.set_source_surface(&buf.content, Z, Z).unwrap();
            ctx.paint().unwrap()
        }
    ));

    let ms = event_handle(
        &darea,
        expose.clone(),
        box_motion_transition.clone(),
        box_ctx.clone(),
    );
    Ok(Box::new(BoxWidgetExpose::new(ms.downgrade(), expose)))
}

fn rotate_content(edge: Edge, content: ImageSurface) -> ImageSurface {
    match edge {
        Edge::Left => content,
        Edge::Right => {
            let size = (content.width(), content.height());
            let surf = new_surface(size);
            let ctx = cairo::Context::new(&surf).unwrap();
            ctx.rotate(-180_f64.to_radians());
            ctx.translate(-size.0 as f64, -size.1 as f64);
            ctx.set_source_surface(content, Z, Z).unwrap();
            ctx.paint().unwrap();
            surf
        }
        Edge::Top => {
            let size = (content.height(), content.width());
            let surf = new_surface(size);
            let ctx = cairo::Context::new(&surf).unwrap();
            ctx.rotate(270.0_f64.to_radians());
            ctx.translate(-size.1 as f64, 0.);
            ctx.set_source_surface(content, Z, Z).unwrap();
            ctx.paint().unwrap();
            surf
        }
        Edge::Bottom => {
            let size = (content.height(), content.width());
            let surf = new_surface(size);
            let ctx = cairo::Context::new(&surf).unwrap();
            ctx.rotate(90.0_f64.to_radians());
            ctx.translate(0., -size.0 as f64);
            ctx.set_source_surface(content, Z, Z).unwrap();
            ctx.paint().unwrap();
            surf
        }
        _ => unreachable!(),
    }
}

fn init_boxed_widgets(bx: &mut GridBox, expose: BoxExposeRc, ws: Vec<BoxedWidgetConfig>) {
    ws.into_iter().for_each(|w| {
        let _ = match w.widget {
            crate::config::Widget::Ring(r) => match init_ring(&expose, *r) {
                Ok(ring) => {
                    bx.add(Rc::new(RefCell::new(ring)), (w.index[0], w.index[1]));
                    Ok(())
                }
                Err(e) => Err(format!("Fail to create ring widget: {e}")),
            },
            crate::config::Widget::Text(t) => match init_text(&expose, *t) {
                Ok(text) => {
                    bx.add(Rc::new(RefCell::new(text)), (w.index[0], w.index[1]));
                    Ok(())
                }
                Err(e) => Err(format!("Fail to create text widget: {e}")),
            },
            _ => unreachable!(),
        }
        .inspect_err(|e| {
            crate::notify_send("Way-edges boxed widgets", e.as_str(), true);
            log::error!("{e}");
        });
    });
}
