use educe::Educe;
use glib::clone::{Downgrade, Upgrade};
use gtk::glib;
use std::{rc::Rc, time::Duration};
use way_edges_derive::wrap_rc;

use crate::{
    animation::ToggleAnimationRc,
    mouse_state::{MouseEvent, MouseStateData},
};

use super::WindowContext;

use gtk::gdk::BUTTON_MIDDLE;

#[wrap_rc(rc = "pub", normal = "pub(super)")]
#[derive(Educe)]
#[educe(Debug)]
pub struct WindowPopState {
    pin_state: bool,
    pop_state: Option<Rc<()>>,
    pop_animation: ToggleAnimationRc,
    pin_key: u32,
    pop_duration: Duration,

    #[educe(Debug(ignore))]
    redraw_signal: Rc<dyn Fn()>,
}
impl WindowPopState {
    pub(super) fn new(ani: ToggleAnimationRc, redraw_signal: Rc<dyn Fn()>) -> Self {
        Self {
            pin_state: false,
            pop_state: None,
            pop_animation: ani,
            pin_key: BUTTON_MIDDLE,
            pop_duration: Duration::from_secs(1),
            redraw_signal,
        }
    }
    fn invalidate_pop(&mut self) {
        drop(self.pop_state.take());
    }
    fn toggle_pin(&mut self) {
        self.invalidate_pop();
        let state = !self.pin_state;
        self.pin_state = state;
        self.pop_animation.borrow_mut().set_direction(state.into());
    }
    fn enter(&mut self) {
        self.invalidate_pop();
        self.pop_animation
            .borrow_mut()
            .set_direction(crate::animation::ToggleDirection::Forward);
    }
    fn leave(&mut self) {
        self.invalidate_pop();
        self.pop_animation
            .borrow_mut()
            .set_direction(crate::animation::ToggleDirection::Backward);
    }
    pub(super) fn progress(&self) -> f64 {
        if self.pin_state {
            1.
        } else {
            self.pop_animation.borrow().progress()
        }
    }
}
impl WindowPopStateRc {
    pub fn pop(&self) {
        let mut s = self.borrow_mut();

        let handle = Rc::new(());
        let handle_weak = Rc::downgrade(&handle);
        s.pop_state = Some(handle);

        s.pop_animation
            .borrow_mut()
            .set_direction(crate::animation::ToggleDirection::Forward);
        (s.redraw_signal)();

        let redraw_weak = Rc::downgrade(&s.redraw_signal);
        let self_weak = Downgrade::downgrade(self);
        let cb = move || {
            if handle_weak.upgrade().is_none() {
                return;
            }
            if let Some(pop_state) = self_weak.upgrade() {
                let Some(r) = redraw_weak.upgrade() else {
                    return;
                };
                let mut s = pop_state.borrow_mut();
                s.invalidate_pop();
                s.pop_animation
                    .borrow_mut()
                    .set_direction(crate::animation::ToggleDirection::Backward);
                r()
            }
        };

        glib::timeout_add_local_once(s.pop_duration, cb);
    }
}

impl WindowContext {
    pub fn toggle_pin(&self) {
        self.window_pop_state.borrow_mut().toggle_pin();
    }
    pub fn setup_mouse_event_callback(
        &mut self,
        mut widget_callback: impl 'static + FnMut(&mut MouseStateData, MouseEvent) -> bool,
    ) {
        let pop_state = &self.window_pop_state;
        let start_pose = &self.start_pos;
        let redraw_func = self.make_redraw_notifier();

        let cb = glib::clone!(
            #[weak]
            pop_state,
            #[weak]
            start_pose,
            move |data: &mut MouseStateData, mut event: MouseEvent| {
                let mut trigger_redraw = false;
                let mut do_redraw = || {
                    if !trigger_redraw {
                        trigger_redraw = true;
                    }
                };

                fn change_pos(pose: &mut (f64, f64), start_pose: (i32, i32)) {
                    pose.0 -= start_pose.0 as f64;
                    pose.1 -= start_pose.1 as f64;
                }

                match &mut event {
                    MouseEvent::Release(pos, _) | MouseEvent::Press(pos, _) => {
                        change_pos(pos, start_pose.get())
                    }
                    MouseEvent::Enter(pos) | MouseEvent::Motion(pos) => {
                        change_pos(pos, start_pose.get())
                    }
                    MouseEvent::Leave => {}
                }

                match event {
                    MouseEvent::Release(_, key) => {
                        let mut pop_state = pop_state.borrow_mut();
                        if key == pop_state.pin_key {
                            pop_state.toggle_pin();
                            do_redraw()
                        };
                    }
                    MouseEvent::Enter(_) => {
                        pop_state.borrow_mut().enter();
                        do_redraw()
                    }
                    MouseEvent::Leave => {
                        pop_state.borrow_mut().leave();
                        do_redraw()
                    }
                    MouseEvent::Motion(_) => pop_state.borrow_mut().invalidate_pop(),
                    _ => {}
                }

                let widget_trigger_redraw = widget_callback(data, event);

                if trigger_redraw || widget_trigger_redraw {
                    redraw_func(None)
                }
            }
        );

        self.mouse_event.borrow_mut().set_event_cb(cb);
    }
}
