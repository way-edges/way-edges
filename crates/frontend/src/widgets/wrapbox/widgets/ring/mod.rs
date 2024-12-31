mod draw;
mod preset;

use std::cell::UnsafeCell;
use std::rc::Rc;

use cairo::ImageSurface;
use educe::Educe;
use gtk::glib;
use interval_task::runner::Runner;

use config::widgets::wrapbox::ring::RingConfig;
use preset::RunnerResult;

use crate::animation::ToggleAnimationRc;
use crate::mouse_state::MouseEvent;
use crate::widgets::wrapbox::box_traits::BoxedWidget;
use crate::widgets::wrapbox::BoxTemporaryCtx;

#[derive(Educe)]
#[educe(Debug)]
pub struct RingCtx {
    #[educe(Debug(ignore))]
    runner: Runner<()>,
    current: Rc<UnsafeCell<RunnerResult>>,
    animation: ToggleAnimationRc,
    #[educe(Debug(ignore))]
    redraw_signal: Box<dyn Fn()>,
}

impl BoxedWidget for RingCtx {
    fn content(&mut self) -> ImageSurface {
        // unsafe { self.cache_content.as_ptr().as_ref().unwrap().clone() }
    }
    fn on_mouse_event(&mut self, event: MouseEvent) {
        match event {
            MouseEvent::Enter(_) => {
                self.animation
                    .borrow_mut()
                    .set_direction(crate::animation::ToggleDirection::Forward);
                (self.redraw_signal)();
            }
            MouseEvent::Leave => {
                self.animation
                    .borrow_mut()
                    .set_direction(crate::animation::ToggleDirection::Backward);
                (self.redraw_signal)();
            }
            _ => {}
        }
    }
}
impl Drop for RingCtx {
    fn drop(&mut self) {
        std::mem::take(&mut self.runner).close().unwrap();
    }
}

pub fn init_ring(box_temp_ctx: &mut BoxTemporaryCtx, conf: RingConfig) -> impl BoxedWidget {
    // let drawer = TextDrawer::new(&conf);

    // runner
    let (mut runner, r) = preset::parse_preset(conf.preset);
    let current = Rc::new(UnsafeCell::new(RunnerResult::default()));
    let current_weak = Rc::downgrade(&current);
    let redraw_signal = box_temp_ctx.make_redraw_signal();
    glib::spawn_future_local(async move {
        while let Ok(res) = r.recv().await {
            let current = current_weak.upgrade().unwrap();
            unsafe { *current.get().as_mut().unwrap() = res };
            redraw_signal();
        }
    });
    runner.start().unwrap();

    let animation = box_temp_ctx.new_animation(conf.text_transition_ms);
    let redraw_signal = Box::new(box_temp_ctx.make_redraw_signal());

    RingCtx {
        runner,
        current,
        animation,
        redraw_signal,
    }
}
