use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gio::glib::clone::Downgrade;
use gio::glib::WeakRef;
use gtk::prelude::{GtkWindowExt, WidgetExt};
use gtk::DrawingArea;
use gtk::{glib, ApplicationWindow};
use interval_task::runner::{ExternalRunnerExt, Runner, Task};

pub struct FrameManager {
    runner: Rc<Cell<Option<Runner<Task>>>>,
    frame_gap: Duration,
    darea: WeakRef<DrawingArea>,
    appwindow: WeakRef<ApplicationWindow>,
}
impl FrameManager {
    pub fn new(frame_rate: u32, darea: &DrawingArea, appwindow: &ApplicationWindow) -> Self {
        let runner = Rc::new(Cell::new(None));
        darea.connect_destroy(glib::clone!(
            #[strong]
            runner,
            move |_| {
                log::debug!("frame manager close");
                runner.take().map(|r: Runner<Task>| r.close());
            }
        ));
        Self {
            runner,
            frame_gap: Duration::from_micros(1_000_000 / frame_rate as u64),
            darea: darea.downgrade(),
            appwindow: appwindow.downgrade(),
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
                #[strong(rename_to=darea)]
                self.darea,
                async move {
                    log::debug!("start wait runner signal");
                    while r.recv().await.is_ok() {
                        if let Some(darea) = darea.upgrade() {
                            darea.queue_draw();
                        } else {
                            log::info!("drawing area is cleared");
                            r.close();
                            break;
                        };
                    }
                    log::debug!("stop wait runner signal");
                }
            ));
        }
        Ok(())
    }
    pub fn stop(&mut self) -> Result<(), String> {
        if let Some(runner) = self.runner.take() {
            log::debug!("stop frame manager");
            glib::spawn_future_local(glib::clone!(
                #[strong(rename_to=window)]
                self.appwindow,
                async move {
                    if let Err(s) = runner.close() {
                        log::error!("Error closing runner: {s}");
                        if let Some(window) = window.upgrade() {
                            window.close();
                        };
                    };
                }
            ));
            log::debug!("runner closed");
        }
        Ok(())
    }
}
