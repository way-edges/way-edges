use std::time::Duration;

use gio::glib::clone::Downgrade;
use gio::glib::WeakRef;
use gtk::prelude::{GtkWindowExt, WidgetExt};
use gtk::DrawingArea;
use gtk::{glib, ApplicationWindow};
use interval_task::runner::{self, ExternalRunnerExt};

pub struct FrameManager {
    runner: Option<runner::Runner<runner::Task>>,
    frame_gap: Duration,
    darea: WeakRef<DrawingArea>,
    appwindow: WeakRef<ApplicationWindow>,
}
impl FrameManager {
    pub fn new(frame_rate: u32, darea: &DrawingArea, appwindow: &ApplicationWindow) -> Self {
        Self {
            runner: None,
            frame_gap: Duration::from_micros(1_000_000 / frame_rate as u64),
            darea: darea.downgrade(),
            appwindow: appwindow.downgrade(),
        }
    }
    pub fn start(&mut self) -> Result<(), String> {
        log::debug!("start frame manager");
        if self.runner.is_none() {
            log::debug!("new runner");
            let (r, mut runner) = interval_task::channel::new(self.frame_gap);
            runner.start()?;
            self.runner = Some(runner);
            log::debug!("runner started");
            glib::spawn_future_local(glib::clone!(@strong self.darea as darea => async move {
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
            }));
        }
        Ok(())
    }
    pub fn stop(&mut self) -> Result<(), String> {
        log::debug!("stop frame manager");
        if let Some(runner) = self.runner.take() {
            log::debug!("close runner start");
            glib::spawn_future_local(
                glib::clone!(@strong self.appwindow as window => async move {
                    if let Err(s) = runner.close() {
                        log::error!("Error closing runner: {s}");
                        if let Some(window) = window.upgrade() {
                            window.close();
                        };
                    };
                }),
            );
            log::debug!("runner closed");
        }
        Ok(())
    }
}
