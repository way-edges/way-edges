use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};

use gtk::glib;
// use interval_task::runner::{ExternalRunnerExt, Runner, Task};
use tokio::sync::oneshot::error::TryRecvError;

type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
type FutureSender = async_channel::Sender<BoxFuture>;

fn get_future_sender() -> &'static FutureSender {
    static FUTURE_SENDER: OnceLock<FutureSender> = OnceLock::new();

    FUTURE_SENDER.get_or_init(|| {
        let (started_signal_sender, started_signal_receiver) = tokio::sync::oneshot::channel();
        let (future_sender, future_receiver) = async_channel::bounded::<BoxFuture>(1);
        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            let spawn_future = |fut: BoxFuture| {
                rt.spawn(fut);
            };
            rt.block_on(async move {
                started_signal_sender.send(()).unwrap();

                while let Ok(fut) = future_receiver.recv().await {
                    spawn_future(fut);
                }
            });
        });
        started_signal_receiver.blocking_recv().unwrap();
        future_sender
    })
}

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
    get_future_sender().send_blocking(Box::pin(fut)).unwrap();
    (stop_sender, signal_receiver)
}

pub type FrameManagerCb = Box<dyn FnMut() + 'static>;
pub type FrameManagerCbRc = Rc<RefCell<FrameManagerCb>>;

pub struct FrameManager {
    runner: Rc<Cell<Option<tokio::sync::oneshot::Sender<()>>>>,
    frame_gap: Duration,
    cb: FrameManagerCbRc,
}
impl FrameManager {
    pub fn new(frame_rate: u32, cb: impl FnMut() + 'static) -> Self {
        let runner = Rc::new(Cell::new(None));
        Self {
            runner,
            frame_gap: Duration::from_micros(1_000_000 / frame_rate as u64),
            cb: Rc::new(RefCell::new(Box::new(cb))),
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
            let (stop_sender, signal_receiver) = add_frame_manage_future(self.frame_gap);
            self.runner.set(Some(stop_sender));
            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to=cb)]
                self.cb,
                async move {
                    log::debug!("start wait runner signal");
                    while signal_receiver.recv().await.is_ok() {
                        cb.borrow_mut()();
                    }
                    log::debug!("stop wait runner signal");
                }
            ));
        }
        Ok(())
    }
    pub fn stop(&mut self) -> Result<(), String> {
        if let Some(runner) = self.runner.take() {
            // runner.send(()).unwrap();
            let _ = runner.send(());
            // runner.close().unwrap();
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
