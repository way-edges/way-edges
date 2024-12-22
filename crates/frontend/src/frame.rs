use std::os::unix::io::AsRawFd;
use std::{cell::Cell, rc::Rc, time::Duration};

use gtk::glib;
use gtk::prelude::WidgetExt;
use gtk::DrawingArea;
use timerfd::TimerFd;

#[derive(Debug)]
pub struct FrameManager {
    frame_gap: Duration,
    current: Option<Rc<Cell<bool>>>,
}
impl FrameManager {
    pub fn new(frame_rate: u64) -> Self {
        Self {
            frame_gap: Duration::from_micros(1_000_000 / frame_rate),
            current: None,
        }
    }
    pub fn start(&mut self, drawing_area: &DrawingArea) {
        if self.current.is_some() {
            return;
        }

        let tfd = self.new_timerfd();
        let handle = Rc::new(Cell::new(false));
        glib::unix_fd_add_local(
            tfd.as_raw_fd(),
            glib::IOCondition::IN,
            glib::clone!(
                #[strong]
                handle,
                #[weak]
                drawing_area,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move |_, _| {
                    if handle.get() {
                        return glib::ControlFlow::Break;
                    }

                    if tfd.read() != 0 {
                        drawing_area.queue_draw();
                    }
                    glib::ControlFlow::Continue
                }
            ),
        );
        self.current = Some(handle);
    }
    pub fn stop(&mut self) {
        if let Some(handle) = self.current.take() {
            handle.set(true);
        }
    }

    fn new_timerfd(&self) -> TimerFd {
        let mut tfd =
            timerfd::TimerFd::new_custom(timerfd::ClockId::Monotonic, true, true).unwrap();
        tfd.set_state(
            timerfd::TimerState::Periodic {
                current: self.frame_gap,
                interval: self.frame_gap,
            },
            timerfd::SetTimeFlags::Default,
        );
        tfd
    }
}
impl Drop for FrameManager {
    fn drop(&mut self) {
        self.stop();
    }
}
