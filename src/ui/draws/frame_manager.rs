use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk::glib;
use tokio::sync::oneshot::error::TryRecvError;

use crate::get_main_runtime_handle;

use super::transition_state::TransitionStateList;

fn add_frame_manage_future(
    interval: Duration,
) -> (
    tokio::sync::oneshot::Sender<()>,
    async_channel::Receiver<()>,
) {
    let (signal_sender, signal_receiver) = async_channel::bounded(1);
    let (stop_sender, mut stop_receiver) = tokio::sync::oneshot::channel::<()>();

    let fut = async move {
        while let Err(TryRecvError::Empty) = stop_receiver.try_recv() {
            let frame_start = Instant::now();

            if signal_sender.force_send(()).is_err() {
                break;
            }

            if let Some(gap) = interval.checked_sub(frame_start.elapsed()) {
                if tokio_timerfd::sleep(gap).await.is_err() {
                    break;
                }
            }
        }
    };

    let handle = get_main_runtime_handle();
    handle.spawn(fut);

    // get_future_sender().send_blocking(Box::pin(fut)).unwrap();
    (stop_sender, signal_receiver)
}

pub type FrameManagerCb = Box<dyn FnMut() + 'static>;
pub type FrameManagerCbRc = Rc<RefCell<FrameManagerCb>>;

pub struct FrameManager {
    stop_sender: Rc<Cell<Option<tokio::sync::oneshot::Sender<()>>>>,
    frame_gap: Duration,
    cb: FrameManagerCbRc,
}
impl FrameManager {
    pub fn new(frame_rate: u32, cb: impl FnMut() + 'static) -> Self {
        Self {
            stop_sender: Rc::new(Cell::new(None)),
            frame_gap: Duration::from_micros(1_000_000 / frame_rate as u64),
            cb: Rc::new(RefCell::new(Box::new(cb))),
        }
    }
    pub fn start(&mut self) -> Result<(), String> {
        let no_fm = unsafe {
            let ptr = self.stop_sender.as_ptr();
            ptr.as_ref().unwrap().is_none()
        };
        if no_fm {
            log::debug!("start frame manager");
            let (stop_sender, signal_receiver) = add_frame_manage_future(self.frame_gap);
            self.stop_sender.set(Some(stop_sender));
            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to=cb)]
                self.cb,
                async move {
                    log::debug!("start wait frame_manage signal");
                    while signal_receiver.recv().await.is_ok() {
                        cb.borrow_mut()();
                    }
                    log::debug!("stop wait frame_manage signal");
                }
            ));
        }
        Ok(())
    }
    pub fn stop(&mut self) -> Result<(), String> {
        if let Some(stop_sender) = self.stop_sender.take() {
            let _ = stop_sender.send(());
            log::debug!("frame manage closed");
        }
        Ok(())
    }
}
impl Drop for FrameManager {
    fn drop(&mut self) {
        self.stop().unwrap();
    }
}

pub trait FrameManagerBindTransition {
    fn ensure_frame_run(&mut self, ts_list: &TransitionStateList);
}
impl FrameManagerBindTransition for FrameManager {
    fn ensure_frame_run(&mut self, ts_list: &TransitionStateList) {
        let is_in_transition = ts_list.iter().any(|f| {
            if let Some(f) = f {
                f.borrow().is_in_transition()
            } else {
                false
            }
        });
        if is_in_transition {
            self.start().unwrap();
        } else {
            self.stop().unwrap();
        }
    }
}
