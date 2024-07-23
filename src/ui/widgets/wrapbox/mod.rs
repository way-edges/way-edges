use std::cell::Cell;
use std::{cell::RefCell, rc::Rc};

use async_channel::{Receiver, Sender};
use gtk::prelude::DrawingAreaExtManual;
use gtk::prelude::WidgetExt;
use gtk::DrawingArea;

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

use gtk::glib;

use crate::ui::draws::util::Z;
pub fn draw() {
    let darea = DrawingArea::new();
    let mut disp = display::grid::GridBox::new(10.);
    let (expose, update_signal_receiver) = BoxExpose::new();
    {
        let buf = Rc::new(Cell::new(None));
        glib::spawn_future_local(glib::clone!(
            #[weak]
            darea,
            #[weak]
            buf,
            async move {
                while (update_signal_receiver.recv().await).is_ok() {
                    let content = disp.draw_content();
                    darea.set_size_request(content.width(), content.height());
                    buf.set(Some(content));
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
}

fn event_handle(darea: &DrawingArea, expose: BoxExposeRc) {}
