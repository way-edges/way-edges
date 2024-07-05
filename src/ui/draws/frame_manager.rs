use std::time::Duration;

use gtk::glib;
use gtk::prelude::WidgetExt;
use gtk::DrawingArea;
use interval_task::runner::{self, ExternalRunnerExt};

pub struct FrameManager {
    runner: Option<runner::Runner<runner::Task>>,
    frame_gap: Duration,
}
impl FrameManager {
    pub fn new(frame_rate: u64) -> Self {
        Self {
            runner: None,
            frame_gap: Duration::from_micros(1_000_000 / frame_rate),
        }
    }
    pub fn start(&mut self, darea: &DrawingArea) -> Result<(), String> {
        if self.runner.is_none() {
            let (r, mut runner) = interval_task::channel::new(self.frame_gap);
            runner.start()?;
            self.runner = Some(runner);
            glib::spawn_future_local(glib::clone!(@weak darea => async move {
                while r.recv().await.is_ok() {
                    darea.queue_draw();
                }
            }));
        }
        Ok(())
    }
    pub fn stop(&mut self) -> Result<(), String> {
        if let Some(runner) = self.runner.take() {
            runner.close()?;
        }
        Ok(())
    }
}
