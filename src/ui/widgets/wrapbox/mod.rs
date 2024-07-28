use std::cell::Cell;
use std::str::FromStr;
use std::time::Duration;
use std::{cell::RefCell, rc::Rc};

use async_channel::{Receiver, Sender};
use cairo::{ImageSurface, RectangleInt, Region};
use display::grid::{BoxedWidgetRc, GridBox, GridItemSizeMapRc};
use gtk::gdk::RGBA;
use gtk::glib;
use gtk::prelude::NativeExt;
use gtk::prelude::{DrawingAreaExtManual, GtkWindowExt, SurfaceExt, WidgetExt};
use gtk::DrawingArea;
use gtk4_layer_shell::Edge;
use outlook::window::BoxOutlookWindowRc;

use crate::ui::draws::frame_manager::FrameManager;
use crate::ui::draws::mouse_state::{
    new_mouse_event_func, new_mouse_state, new_translate_mouse_state, MouseEvent,
};
use crate::ui::draws::transition_state::{self, TransitionState, TransitionStateRc};
use crate::ui::draws::util::{draw_motion, draw_rotation, ensure_frame_manager, new_surface, Z};

use super::ring::init_ring;

pub mod display;
pub mod outlook;

pub type UpdateSignal = ();
pub type MousePosition = (f64, f64);
pub type BoxExposeRc = Rc<RefCell<BoxExpose>>;

pub struct BoxExpose {
    pub update_signal: Sender<UpdateSignal>,
}

impl BoxExpose {
    fn new() -> (BoxExposeRc, Receiver<UpdateSignal>) {
        let (update_signal_sender, update_signal_receiver) = async_channel::bounded(1);
        let se = Rc::new(RefCell::new(BoxExpose {
            update_signal: update_signal_sender,
        }));
        (se, update_signal_receiver)
    }
    pub fn update_func(&self) -> impl Fn() + Clone {
        let s = self.update_signal.downgrade();
        move || {
            if let Some(s) = s.upgrade() {
                // ignored result
                s.force_send(()).ok();
            }
        }
    }
}

struct BoxBuffer {
    content: ImageSurface,
    y: f64,
}

pub fn init_widget(window: &gtk::ApplicationWindow) {
    let edge = Edge::Bottom;
    let position = Edge::Bottom;
    let extra_trigger_size = 5.;

    let darea = DrawingArea::new();

    let (mut ol, expose, mut disp, update_signal_receiver) = {
        let ol = outlook::window::BoxOutlookWindow::new(
            None,
            RGBA::from_str("#C18F4A").unwrap(),
            None,
            10.,
            20.,
        );
        let mut disp = display::grid::GridBox::new(10.);
        let (expose, update_signal_receiver) = BoxExpose::new();
        init_boxed_widgets(&mut disp, expose.clone());
        (ol, expose, disp, update_signal_receiver)
    };

    let (box_motion_transition, mut box_frame_manager) = {
        let ts = Rc::new(RefCell::new(TransitionState::new(Duration::from_millis(
            100,
        ))));
        let fm = {
            let up = expose.borrow().update_func();
            FrameManager::new(
                90,
                glib::clone!(move || {
                    up();
                }),
            )
        };
        (ts, fm)
    };

    let (outlook_rc, buf, filtered_grid_item_map, input_region) = {
        let set_size = glib::clone!(
            #[weak]
            darea,
            move |size: (i32, i32)| {
                darea.set_size_request(size.0, size.1);
            }
        );
        let match_size = move |wh: (i32, i32)| -> (i32, i32) {
            match edge {
                Edge::Left | Edge::Right => wh,
                Edge::Top | Edge::Bottom => (wh.1, wh.0),
                _ => unreachable!(),
            }
        };
        let match_rect = move |darea: &DrawingArea, wh: (i32, i32), y: f64| -> RectangleInt {
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
                | (Edge::Bottom, Edge::Right) => darea.width() - wh.0,
                (Edge::Top, Edge::Top)
                | (Edge::Top, Edge::Bottom)
                | (Edge::Bottom, Edge::Top)
                | (Edge::Bottom, Edge::Bottom) => (darea.width() - wh.0) / 2,
                _ => unreachable!(),
            };
            let (y, h) = match edge {
                Edge::Top => (Z as i32, (wh.1 as f64 * y) as i32),
                Edge::Bottom => ((wh.1 as f64 * (1. - y)) as i32, (wh.1 as f64 * y) as i32),
                _ => (Z as i32, wh.1),
            };
            RectangleInt::new(x, y, wh.0, h)
        };
        let (outlook_rc, buf, filtered_grid_item_map, input_region) = {
            let (content, filtered_grid_item_map, rectint) = {
                let (content, filtered_grid_item_map) = disp.draw_content();
                ol.redraw((content.width() as f64, content.height() as f64));
                let content = ol.with_box(content);
                let wh = match_size((content.width(), content.height()));
                set_size(wh);
                let rectint = match_rect(&darea, wh, box_motion_transition.borrow().get_y());
                (content, filtered_grid_item_map, rectint)
            };
            let buffer = BoxBuffer {
                content,
                y: box_motion_transition.borrow().get_y(),
            };
            (
                Rc::new(RefCell::new(ol)),
                Rc::new(Cell::new(Some(buffer))),
                Rc::new(Cell::new(filtered_grid_item_map)),
                Rc::new(Cell::new(rectint)),
            )
        };
        // it's a async block once, doesn't matter strong or weak
        glib::spawn_future_local(glib::clone!(
            #[weak]
            darea,
            #[weak]
            window,
            #[strong]
            buf,
            #[strong]
            filtered_grid_item_map,
            #[strong]
            outlook_rc,
            #[strong]
            box_motion_transition,
            #[strong]
            input_region,
            async move {
                log::debug!("box draw signal receive loop start");
                while (update_signal_receiver.recv().await).is_ok() {
                    let (content, filtered_map) = disp.draw_content();
                    let content = rotate_content(edge, content);
                    let content = {
                        let content_size = (content.width(), content.height());
                        let mut ol = outlook_rc.borrow_mut();
                        let size = ol.cache.as_ref().unwrap().content_size;
                        let size = (size.0 as i32, size.1 as i32);
                        if size != content_size {
                            ol.redraw((content_size.0 as f64, content_size.1 as f64));
                        }
                        ol.with_box(content)
                    };
                    let wh = match_size((content.width(), content.height()));
                    set_size(wh);
                    let y = box_motion_transition.borrow().get_y();
                    let buffer = {
                        ensure_frame_manager(&mut box_frame_manager, y);
                        BoxBuffer { content, y }
                    };
                    buf.set(Some(buffer));

                    let mut inr = match_rect(&darea, wh, y);
                    input_region.set(inr);

                    match edge {
                        Edge::Top => {
                            inr.set_height(inr.height() + extra_trigger_size as i32);
                        }
                        Edge::Bottom => {
                            inr.set_y(inr.y() - extra_trigger_size as i32);
                            inr.set_height(inr.height() + extra_trigger_size as i32);
                        }
                        _ => {}
                    }

                    if let Some(surf) = window.surface() {
                        surf.set_input_region(&Region::create_rectangle(&inr));
                    }

                    filtered_grid_item_map.set(filtered_map);
                    darea.queue_draw();
                }
                log::debug!("box draw signal receive loop exit");
            }
        ));
        (outlook_rc, buf, filtered_grid_item_map, input_region)
    };

    darea.set_draw_func(glib::clone!(
        #[weak]
        window,
        move |darea, ctx, _, _| {
            if let Some(buf) = buf.take() {
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
                draw_motion(ctx, visible_y, edge, range, extra_trigger_size);

                ctx.set_source_surface(&buf.content, Z, Z).unwrap();
                ctx.paint().unwrap()
            }
        }
    ));

    event_handle(
        &darea,
        expose.clone(),
        filtered_grid_item_map,
        outlook_rc,
        box_motion_transition,
        input_region,
    );
    darea.connect_destroy(move |_| {
        log::debug!("DrawingArea destroyed");
    });
    window.connect_destroy(move |_| {
        log::debug!("destroy window");
        expose.borrow().update_signal.close();
    });
    window.set_child(Some(&darea));
}

fn event_handle(
    darea: &DrawingArea,
    expose: BoxExposeRc,
    filtered_grid_item_map: GridItemSizeMapRc,
    outlook_rc: BoxOutlookWindowRc,
    ts: TransitionStateRc,
    input_region: Rc<Cell<RectangleInt>>,
) {
    let ms = new_mouse_state(darea);
    let mut last_widget: Option<BoxedWidgetRc> = None;
    let cb = {
        let f = expose.borrow().update_func();
        new_mouse_event_func(move |e| {
            match e {
                MouseEvent::Enter(pos) | MouseEvent::Motion(pos) => {
                    let rectint = input_region.as_ref().clone().into_inner();
                    let pos = (pos.0 - rectint.x() as f64, pos.1 - rectint.y() as f64);
                    let pos = outlook_rc.borrow().transform_mouse_pos(pos);
                    let matched = unsafe { filtered_grid_item_map.as_ptr().as_ref().unwrap() }
                        .match_item(pos);
                    if let Some((widget, pos)) = matched {
                        if let Some(last) = last_widget.take() {
                            if Rc::ptr_eq(&last, &widget) {
                                widget.borrow_mut().on_mouse_event(MouseEvent::Motion(pos));
                            } else {
                                last.borrow_mut().on_mouse_event(MouseEvent::Leave);
                                widget.borrow_mut().on_mouse_event(MouseEvent::Enter(pos));
                            }
                        } else {
                            widget.borrow_mut().on_mouse_event(MouseEvent::Enter(pos));
                        }
                        last_widget = Some(widget);
                    } else {
                        if let Some(last) = last_widget.take() {
                            last.borrow_mut().on_mouse_event(MouseEvent::Leave);
                        }
                    }
                    f();
                }
                MouseEvent::Leave => {
                    last_widget = None;
                    f();
                }
                _ => {}
            };
        })
    };
    let cb = new_translate_mouse_state(ts, ms.clone(), Some(cb), false);
    ms.borrow_mut().set_event_cb(cb);
}

fn init_boxed_widgets(bx: &mut GridBox, expose: BoxExposeRc) {
    for i in 0..9 {
        let ring = Rc::new(RefCell::new(init_ring(
            &expose,
            5.,
            5. + i as f64 * 2.,
            RGBA::from_str("#9F9F9F").unwrap(),
            RGBA::from_str("#F1FA8C").unwrap(),
        )));

        let r_idx = i / 3;
        let c_idx = i % 3;
        bx.add(ring, (r_idx, c_idx));
    }
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
