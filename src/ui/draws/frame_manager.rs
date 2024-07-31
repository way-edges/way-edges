use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use gtk::glib;
use interval_task::runner::{ExternalRunnerExt, Runner, Task};

pub type FrameManagerCb = Box<dyn FnMut() + 'static>;
pub type FrameManagerCbRc = Rc<RefCell<FrameManagerCb>>;

pub struct FrameManager {
    runner: Rc<Cell<Option<Runner<Task>>>>,
    frame_gap: Duration,
    cb: FrameManagerCbRc,
    // darea: WeakRef<DrawingArea>,
    // appwindow: WeakRef<ApplicationWindow>,
}
impl FrameManager {
    pub fn new(frame_rate: u32, cb: impl FnMut() + 'static) -> Self {
        let runner = Rc::new(Cell::new(None));
        Self {
            runner,
            frame_gap: Duration::from_micros(1_000_000 / frame_rate as u64),
            // cb: Box::new(cb),
            cb: Rc::new(RefCell::new(Box::new(cb))),
            // darea: darea.downgrade(),
            // appwindow: appwindow.downgrade(),
        }
    }
    pub fn start(&mut self) -> Result<(), String> {
        let is_no_runner = unsafe {
            let ptr = self.runner.as_ptr();
            let runner = ptr.as_ref().unwrap();
            runner.is_none()
        };
        if is_no_runner {
            log::debug!("start frame manager");
            log::debug!("new runner");
            let (r, mut runner) = interval_task::channel::new(self.frame_gap);
            runner.start()?;
            self.runner.set(Some(runner));
            log::debug!("runner started");
            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to=cb)]
                self.cb,
                async move {
                    log::debug!("start wait runner signal");
                    while r.recv().await.is_ok() {
                        cb.borrow_mut()();
                        // if let Some(darea) = darea.upgrade() {
                        //     darea.queue_draw();
                        // } else {
                        //     log::info!("drawing area is cleared");
                        //     r.close();
                        //     break;
                        // };
                    }
                    log::debug!("stop wait runner signal");
                }
            ));
        }
        Ok(())
    }
    pub fn stop(&mut self) -> Result<(), String> {
        if let Some(runner) = self.runner.take() {
            runner.close().unwrap();
            log::debug!("runner closed");
        }
        Ok(())
    }
}
impl Drop for FrameManager {
    fn drop(&mut self) {
        self.stop().unwrap();
    }
}
