use educe::Educe;
use gtk::{
    glib,
    prelude::{GestureSingleExt, WidgetExt},
    DrawingArea, EventControllerMotion, GestureClick,
};
use way_edges_derive::wrap_rc;

#[derive(Debug, Clone)]
pub enum MouseEvent {
    Press((f64, f64), u32),
    Release((f64, f64), u32),
    Enter((f64, f64)),
    Leave,
    Motion((f64, f64)),
}

pub type MouseEventFunc = Box<dyn FnMut(&mut MouseStateData, MouseEvent) + 'static>;

#[derive(Debug)]
pub struct MouseStateData {
    pub hovering: bool,
    pub pressing: Option<u32>,
}
impl MouseStateData {
    pub fn new() -> Self {
        Self {
            hovering: false,
            pressing: None,
        }
    }
}
impl Default for MouseStateData {
    fn default() -> Self {
        Self::new()
    }
}

#[wrap_rc(rc = "pub", normal = "pub")]
#[derive(Educe)]
#[educe(Debug)]
pub struct MouseState {
    data: MouseStateData,
    mouse_debug: bool,
    #[educe(Debug(ignore))]
    cb: Option<MouseEventFunc>,
}
impl MouseState {
    pub fn is_hovering(&self) -> bool {
        self.data.hovering
    }
}

impl MouseState {
    pub fn new() -> Self {
        Self {
            data: MouseStateData::new(),
            mouse_debug: false,
            cb: None,
        }
    }

    fn call_event(&mut self, e: MouseEvent) {
        if let Some(f) = &mut self.cb {
            f(&mut self.data, e)
        }
    }

    // pub fn set_event_cb(&mut self, cb: MouseEventFunc) {
    pub fn set_event_cb(&mut self, cb: impl FnMut(&mut MouseStateData, MouseEvent) + 'static) {
        self.cb.replace(Box::new(cb));
    }

    // triggers
    fn press(&mut self, p: u32, pos: (f64, f64)) {
        if self.mouse_debug {
            let msg = format!("key pressed: {}", p);
            log::debug!("Mouse Debug info: {msg}");
            util::notify_send("Way-edges mouse button debug message", &msg, false);
        };

        if self.data.pressing.is_none() {
            self.data.pressing = Some(p);
            self.call_event(MouseEvent::Press(pos, p));
        }
    }
    fn unpress(&mut self, p: u32, pos: (f64, f64)) {
        if self.mouse_debug {
            let msg = format!("key released: {}", p);
            log::debug!("Mouse Debug info: {msg}");
            util::notify_send("Way-edges mouse button debug message", &msg, false);
        };

        if self.data.pressing.eq(&Some(p)) {
            self.data.pressing = None;
            self.call_event(MouseEvent::Release(pos, p));
        }
    }
    fn hover_enter(&mut self, pos: (f64, f64)) {
        self.data.hovering = true;
        self.call_event(MouseEvent::Enter(pos));
    }
    fn hover_motion(&mut self, pos: (f64, f64)) {
        self.call_event(MouseEvent::Motion(pos));
    }
    fn hover_leave(&mut self) {
        self.data.hovering = false;
        self.call_event(MouseEvent::Leave);
    }
}

impl Default for MouseState {
    fn default() -> Self {
        Self::new()
    }
}
impl MouseState {
    pub fn connect(self, darea: &DrawingArea) -> MouseStateRc {
        let ms = self.make_rc();
        {
            let click_control = GestureClick::builder().button(0).exclusive(true).build();
            click_control.connect_pressed(glib::clone!(
                #[weak]
                ms,
                move |g, _, x, y| {
                    ms.borrow_mut().press(g.current_button(), (x, y));
                }
            ));
            click_control.connect_released(glib::clone!(
                #[weak]
                ms,
                move |g, _, x, y| {
                    ms.borrow_mut().unpress(g.current_button(), (x, y));
                }
            ));
            click_control.connect_unpaired_release(glib::clone!(
                #[weak]
                ms,
                move |_, x, y, d, _| {
                    ms.borrow_mut().unpress(d, (x, y));
                }
            ));
            darea.add_controller(click_control);
        };
        {
            let motion = EventControllerMotion::new();
            motion.connect_enter(glib::clone!(
                #[weak]
                ms,
                move |_, x, y| {
                    ms.borrow_mut().hover_enter((x, y));
                }
            ));
            motion.connect_leave(glib::clone!(
                #[weak]
                ms,
                move |_| {
                    ms.borrow_mut().hover_leave();
                }
            ));
            motion.connect_motion(glib::clone!(
                #[weak]
                ms,
                move |_, x, y| {
                    ms.borrow_mut().hover_motion((x, y));
                }
            ));
            darea.add_controller(motion);
        }
        ms
    }
}
