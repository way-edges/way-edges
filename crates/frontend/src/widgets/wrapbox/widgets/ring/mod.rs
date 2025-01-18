mod draw;
mod preset;

use std::cell::UnsafeCell;
use std::rc::Rc;

use cairo::ImageSurface;
use draw::RingDrawer;
use interval_task::runner::Runner;

use config::widgets::wrapbox::ring::RingConfig;
use preset::RunnerResult;

use crate::mouse_state::MouseEvent;
use crate::widgets::wrapbox::box_traits::BoxedWidget;
use crate::widgets::wrapbox::BoxTemporaryCtx;

#[derive(Debug)]
pub struct RingCtx {
    #[allow(dead_code)]
    runner: Runner<()>,
    current: Rc<UnsafeCell<RunnerResult>>,
    drawer: RingDrawer,
}

impl BoxedWidget for RingCtx {
    fn content(&mut self) -> ImageSurface {
        let current = unsafe { self.current.get().as_ref().unwrap() };
        self.drawer.draw(current)
    }
    fn on_mouse_event(&mut self, event: MouseEvent) -> bool {
        match event {
            MouseEvent::Enter(_) => {
                self.drawer
                    .animation
                    .borrow_mut()
                    .set_direction(crate::animation::ToggleDirection::Forward);
                true
            }
            MouseEvent::Leave => {
                self.drawer
                    .animation
                    .borrow_mut()
                    .set_direction(crate::animation::ToggleDirection::Backward);
                true
            }
            _ => false,
        }
    }
}

pub fn init_widget(box_temp_ctx: &mut BoxTemporaryCtx, mut conf: RingConfig) -> impl BoxedWidget {
    let drawer = RingDrawer::new(box_temp_ctx, &mut conf);

    // runner
    let current = Rc::new(UnsafeCell::new(RunnerResult::default()));
    let current_weak = Rc::downgrade(&current);
    let redraw_signal = box_temp_ctx.make_redraw_channel(move |_, msg| {
        let Some(current) = current_weak.upgrade() else {
            return;
        };
        unsafe { *current.get().as_mut().unwrap() = msg };
    });
    let mut runner = preset::parse_preset(conf.preset, redraw_signal);
    runner.start().unwrap();

    RingCtx {
        runner,
        current,
        drawer,
    }
}
