use std::cell::Cell;
use std::str::FromStr;
use std::time::Duration;
use std::{cell::RefCell, rc::Rc};

use async_channel::{Receiver, Sender};
use cairo::{RectangleInt, Region};
use display::grid::{BoxedWidgetRc, GridItemSizeMapRc};
use gtk::gdk::RGBA;
use gtk::glib;
use gtk::prelude::{DrawingAreaExtManual, GtkWindowExt, NativeExt, SurfaceExt, WidgetExt};
use gtk::DrawingArea;
use outlook::window::BoxOutlookWindowRc;

use crate::ui::draws::mouse_state::{
    new_mouse_event_func, new_mouse_state, new_translate_mouse_state, MouseEvent,
};
use crate::ui::draws::transition_state::TransitionState;
use crate::ui::draws::util::Z;

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

pub fn init_widget(window: &gtk::ApplicationWindow) {
    let darea = DrawingArea::new();
    let mut ol = outlook::window::BoxOutlookWindow::new(
        None,
        RGBA::from_str("#C18F4A").unwrap(),
        None,
        10.,
        20.,
    );
    let mut disp = display::grid::GridBox::new(10.);
    let (expose, update_signal_receiver) = BoxExpose::new();
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
        disp.add(ring, (r_idx, c_idx));
    }
    let set_size = glib::clone!(
        #[weak]
        darea,
        #[weak]
        window,
        move |size: (i32, i32)| {
            darea.set_size_request(size.0, size.1);
            if let Some(surf) = window.surface() {
                let region = Region::create_rectangle(&RectangleInt::new(0, 0, size.0, size.1));
                surf.set_input_region(&region);
            }
        }
    );
    let (outlook_rc, buf, filtered_grid_item_map) = {
        let (content, filtered_grid_item_map) = disp.draw_content();
        ol.redraw((content.width() as f64, content.height() as f64));
        let content = ol.with_box(content.clone());
        set_size((content.width(), content.height()));
        (
            Rc::new(RefCell::new(ol)),
            Rc::new(Cell::new(Some(content))),
            Rc::new(Cell::new(filtered_grid_item_map)),
        )
    };
    glib::spawn_future_local(glib::clone!(
        #[weak]
        darea,
        #[strong]
        buf,
        #[strong]
        filtered_grid_item_map,
        #[strong]
        outlook_rc,
        async move {
            log::debug!("box draw signal receive loop start");
            while (update_signal_receiver.recv().await).is_ok() {
                let (content, filtered_map) = disp.draw_content();
                let content_size = (content.width(), content.height());
                let content = {
                    let mut ol = outlook_rc.borrow_mut();
                    let size = ol.cache.as_ref().unwrap().content_size;
                    let size = (size.0 as i32, size.1 as i32);
                    if size != content_size {
                        ol.redraw((content_size.0 as f64, content_size.1 as f64));
                    }
                    ol.with_box(content)
                };
                set_size((content.width(), content.height()));
                buf.set(Some(content));

                filtered_grid_item_map.set(filtered_map);
                darea.queue_draw();
            }
            log::debug!("box draw signal receive loop exit");
        }
    ));
    darea.set_draw_func(move |_, ctx, _, _| {
        if let Some(content) = buf.take() {
            ctx.set_source_surface(&content, Z, Z).unwrap();
            ctx.paint().unwrap()
        }
    });

    event_handle(
        &darea,
        expose.clone(),
        filtered_grid_item_map.clone(),
        outlook_rc.clone(),
    );
    darea.connect_destroy(move |_| {
        log::debug!("DrawingArea destroyed");
    });
    window.connect_destroy(move |_| {
        log::debug!("destroy window");
        expose.borrow().update_signal.close();
        let _ = &filtered_grid_item_map;
        let _ = &outlook_rc;
    });
    window.set_child(Some(&darea));
}

fn event_handle(
    darea: &DrawingArea,
    expose: BoxExposeRc,
    filtered_grid_item_map: GridItemSizeMapRc,
    outlook_rc: BoxOutlookWindowRc,
) {
    let ms = new_mouse_state(darea);
    let ts = Rc::new(RefCell::new(TransitionState::new(Duration::from_millis(
        100,
    ))));
    let mut last_widget: Option<BoxedWidgetRc> = None;
    let cb = {
        let f = expose.borrow().update_func();
        new_mouse_event_func(move |e| {
            match e {
                MouseEvent::Enter(pos) | MouseEvent::Motion(pos) => {
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
