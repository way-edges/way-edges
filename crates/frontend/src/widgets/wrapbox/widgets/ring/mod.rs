mod draw;
mod preset;

use std::cell::UnsafeCell;
use std::rc::Rc;

use cairo::ImageSurface;
use draw::RingDrawer;
use educe::Educe;
use gtk::glib;
use interval_task::runner::Runner;

use config::widgets::wrapbox::ring::RingConfig;
use preset::RunnerResult;

use crate::mouse_state::MouseEvent;
use crate::widgets::wrapbox::box_traits::BoxedWidget;
use crate::widgets::wrapbox::BoxTemporaryCtx;

#[derive(Educe)]
#[educe(Debug)]
pub struct RingCtx {
    #[educe(Debug(ignore))]
    runner: Runner<()>,
    current: Rc<UnsafeCell<RunnerResult>>,
    #[educe(Debug(ignore))]
    redraw_signal: Box<dyn Fn()>,

    drawer: RingDrawer,
}

impl BoxedWidget for RingCtx {
    fn content(&mut self) -> ImageSurface {
        let current = unsafe { self.current.get().as_ref().unwrap() };
        self.drawer.draw(current)
    }
    fn on_mouse_event(&mut self, event: MouseEvent) {
        match event {
            MouseEvent::Enter(_) => {
                self.drawer
                    .animation
                    .borrow_mut()
                    .set_direction(crate::animation::ToggleDirection::Forward);
                (self.redraw_signal)();
            }
            MouseEvent::Leave => {
                self.drawer
                    .animation
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

pub fn init_widget(box_temp_ctx: &mut BoxTemporaryCtx, mut conf: RingConfig) -> impl BoxedWidget {
    let drawer = RingDrawer::new(box_temp_ctx, &mut conf);

    // runner
    let (mut runner, r) = preset::parse_preset(conf.preset);
    let current = Rc::new(UnsafeCell::new(RunnerResult::default()));
    let current_weak = Rc::downgrade(&current);
    let redraw_signal = box_temp_ctx.make_redraw_signal();
    glib::spawn_future_local(async move {
        while let Ok(res) = r.recv().await {
            let Some(current) = current_weak.upgrade() else {
                break;
            };
            unsafe { *current.get().as_mut().unwrap() = res };
            redraw_signal();
        }
    });
    runner.start().unwrap();

    let redraw_signal = Box::new(box_temp_ctx.make_redraw_signal());

    RingCtx {
        runner,
        current,
        redraw_signal,
        drawer,
    }
}
