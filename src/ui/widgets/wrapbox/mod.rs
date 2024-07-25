use std::cell::Cell;
use std::str::FromStr;
use std::time::Duration;
use std::{cell::RefCell, rc::Rc};

use async_channel::{Receiver, Sender};
// use display::grid::{get_item_from_filtered_grid_map_rc, FilteredGridItemMapRc, GridItemSizeMap};
use display::grid::{GridItemSizeMap, GridItemSizeMapRc};
use gtk::gdk::RGBA;
use gtk::glib;
use gtk::prelude::{DrawingAreaExtManual, GtkWindowExt, WidgetExt};
use gtk::DrawingArea;

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
    update_signal: Sender<UpdateSignal>,
    motion_cbs: Vec<Box<dyn FnMut(MousePosition)>>,
    enter_cbs: Vec<Box<dyn FnMut(MousePosition)>>,
    leave_cbs: Vec<Box<dyn FnMut()>>,
    press_cbs: Vec<Box<dyn FnMut(MousePosition)>>,
    release_cbs: Vec<Box<dyn FnMut(MousePosition)>>,
}

impl BoxExpose {
    fn new() -> (BoxExposeRc, Receiver<UpdateSignal>) {
        let (update_signal_sender, update_signal_receiver) = async_channel::bounded(1);
        let se = Rc::new(RefCell::new(BoxExpose {
            update_signal: update_signal_sender,
            motion_cbs: vec![],
            enter_cbs: vec![],
            leave_cbs: vec![],
            press_cbs: vec![],
            release_cbs: vec![],
        }));
        (se, update_signal_receiver)
    }
    pub fn update_signal(&self) -> Sender<UpdateSignal> {
        self.update_signal.clone()
    }
    pub fn update_func(&self) -> impl Fn() + Clone {
        let s = self.update_signal.clone();
        move || {
            s.force_send(());
        }
    }
    pub fn on_motion(&mut self, cb: impl FnMut(MousePosition) + 'static) {
        self.motion_cbs.push(Box::new(cb));
    }
    pub fn on_enter(&mut self, cb: impl FnMut(MousePosition) + 'static) {
        self.enter_cbs.push(Box::new(cb));
    }
    pub fn on_leave(&mut self, cb: impl FnMut() + 'static) {
        self.leave_cbs.push(Box::new(cb));
    }
    pub fn on_press(&mut self, cb: impl FnMut(MousePosition) + 'static) {
        self.press_cbs.push(Box::new(cb));
    }
    pub fn on_release(&mut self, cb: impl FnMut(MousePosition) + 'static) {
        self.release_cbs.push(Box::new(cb));
    }
}

pub fn init_widget(window: &gtk::ApplicationWindow) {
    let darea = DrawingArea::new();
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
    let (buf, filtered_grid_item_map) = {
        let (content, filtered_grid_item_map) = disp.draw_content();
        darea.set_size_request(content.width(), content.height());
        (
            Rc::new(Cell::new(Some(content))),
            // Rc::new(RefCell::new(buf_grid_size_map)),
            Rc::new(Cell::new(filtered_grid_item_map)),
        )
    };
    {
        glib::spawn_future_local(glib::clone!(
            #[weak]
            darea,
            #[weak]
            buf,
            #[weak]
            filtered_grid_item_map,
            async move {
                while (update_signal_receiver.recv().await).is_ok() {
                    let (content, filtered_map) = disp.draw_content();
                    darea.set_size_request(content.width(), content.height());
                    buf.set(Some(content));
                    filtered_grid_item_map.set(filtered_map);
                    darea.queue_draw();
                }
            }
        ));
        darea.set_draw_func(move |_, ctx, _, _| {
            if let Some(content) = buf.take() {
                ctx.set_source_surface(&content, Z, Z).unwrap();
                ctx.paint().unwrap()
            }
        });
    }
    event_handle(&darea, expose, filtered_grid_item_map);
    window.set_child(Some(&darea));
}

fn event_handle(
    darea: &DrawingArea,
    expose: BoxExposeRc,
    filtered_grid_item_map: GridItemSizeMapRc,
) {
    let ms = new_mouse_state(darea);
    let ts = Rc::new(RefCell::new(TransitionState::new(Duration::from_millis(
        100,
    ))));
    let f = expose.borrow().update_func();
    let cb = {
        let f = f.clone();
        new_mouse_event_func(move |e| {
            match e {
                MouseEvent::Enter(_) | MouseEvent::Leave => {
                    f();
                }
                MouseEvent::Motion(pos) => {
                    f();
                    let matched = unsafe { filtered_grid_item_map.as_ptr().as_ref().unwrap() }
                        .match_item(pos);
                    if let Some((widget, pos)) = matched {
                        println!("{pos:?}, {widget:?}");
                    }
                }
                _ => {}
            };
        })
    };
    let cb = new_translate_mouse_state(ts, ms.clone(), Some(cb), false);
    ms.borrow_mut().set_event_cb(cb);
}
