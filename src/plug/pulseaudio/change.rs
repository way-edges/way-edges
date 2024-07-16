use std::{
    cell::RefCell,
    hint::spin_loop,
    rc::Rc,
    sync::{
        atomic::{AtomicPtr, Ordering},
        Arc, RwLock,
    },
};

use async_channel::Sender;
use gtk::glib;
use libpulse_binding::{
    self as pulse,
    context::{self, Context, FlagSet, State},
    mainloop::threaded::Mainloop,
    volume::Volume,
};

use crate::plug::pulseaudio::pa::{get_default_sink, get_default_source, SinkOrSourceInfo};

use super::{OptionalSinkOrSource, OptionalSinkOrSourceDevice};

// pamixser thread
type PaInfoSignal = (OptionalSinkOrSource, VolOrMute);
pub enum VolOrMute {
    Volume(f64),
    Mute(bool),
}
impl VolOrMute {
    pub fn vol(v: f64) -> Self {
        Self::Volume(v)
    }
    pub fn mute(m: bool) -> Self {
        Self::Mute(m)
    }
}

static PA_INFO_SENDER: AtomicPtr<Sender<PaInfoSignal>> = AtomicPtr::new(std::ptr::null_mut());

// struct Debouncer {
//     timer: Rc<Cell<Instant>>,
//     last_signal: Rc<Cell<bool>>,
//     gap: Duration,
// }
// impl Debouncer {
//     fn new(gap: Duration, immediate: bool) -> Self {
//         let mut timer = Instant::now();
//         if immediate {
//             timer = timer.checked_sub(gap).unwrap();
//         }
//         let timer = Rc::new(Cell::new(timer));
//         Self {
//             timer,
//             last_signal: Rc::new(Cell::new(true)),
//             gap,
//         }
//     }
//     fn _is_time_up(t: Rc<Cell<Instant>>, gap: Duration) -> bool {
//         if t.get().elapsed() > gap {
//             t.set(Instant::now());
//             true
//         } else {
//             false
//         }
//     }
//     fn is_time_up(&self) -> bool {
//         Debouncer::_is_time_up(self.timer.clone(), self.gap)
//     }
//     fn update_last_signal(&mut self) -> Rc<Cell<bool>> {
//         self.last_signal.set(true);
//         let n = Rc::new(Cell::new(false));
//         self.last_signal = n.clone();
//         n
//     }
//     fn run(&mut self, task: impl FnOnce() + 'static) {
//         if self.is_time_up() {
//             task();
//         } else {
//             let s = self.update_last_signal();
//             let t = self.timer.clone();
//             let g = self.gap;
//             glib::timeout_add_local_once(Duration::from_millis(10), move || {
//                 if !s.get() {
//                     Debouncer::_is_time_up(t, g);
//                     task()
//                 }
//             });
//         }
//     }
// }
//
// // this will only used by on function in one thread(glib mainloop)
// // so i just use unsafe
// static mut DEBUNCER: Option<Debouncer> = None;
// fn get_debouncer() -> &'static mut Debouncer {
//     unsafe {
//         if DEBUNCER.is_none() {
//             DEBUNCER = Some(Debouncer::new(Duration::from_millis(10), true))
//         }
//         DEBUNCER.as_mut().unwrap()
//     }
// }

pub fn send_painfo_change_signal(msg: PaInfoSignal) {
    glib::spawn_future_local(async move {
        log::debug!("send signal");
        unsafe {
            PA_INFO_SENDER
                .load(Ordering::Acquire)
                .as_ref()
                .unwrap()
                // .send_blocking(msg)
                .send(msg)
                .await
        };
        log::debug!("send signal done");
    });
    // get_debouncer().run(move || {
    // });
}

pub fn init_painfo_changer() {
    let (s, r) = async_channel::bounded::<PaInfoSignal>(1);
    glib::spawn_future_local(async move {
        // thread::spawn(move || {
        let pat = Rc::new(
            PulseAudioThing::new().expect("Fail to init pulseaudio mainloop for modification"),
        );
        loop {
            log::debug!("wait for painfo signal");
            let res = r.recv().await;
            // let res = r.recv_blocking();
            log::debug!("process painfo signal");
            match res {
                Ok((sink_or_source, info)) => {
                    // pamixer_cmd(sink_or_source, info);
                    match sink_or_source.0.as_ref() {
                        OptionalSinkOrSourceDevice::Sink(os) => {
                            let name = os.as_ref().or(get_default_sink());
                            if let Some(name) = name {
                                set_sink(pat.clone(), name, info);
                            } else {
                                log::error!(
                                    "device not found for pulseaudio given sink description: {sink_or_source:?}"
                                );
                            }
                        }
                        OptionalSinkOrSourceDevice::Source(os) => {
                            let name = os.as_ref().or(get_default_source());
                            if let Some(name) = name {
                                set_source(pat.clone(), name, info);
                            } else {
                                log::error!(
                                    "device not found for pulseaudio given source description: {sink_or_source:?}"
                                );
                            }
                        }
                    };
                    log::debug!("set painfo signal done");
                }
                Err(e) => {
                    log::error!("Error receiving pamixser signal, quiting: {e}");
                    break;
                }
            }
        }
    });
    // thread::spawn(move || loop {});

    PA_INFO_SENDER.store(Box::into_raw(Box::new(s)), Ordering::Release);
}

struct PulseAudioThing {
    ml: Rc<RefCell<Mainloop>>,
    ctx: Rc<RefCell<Context>>,
}
impl PulseAudioThing {
    fn new() -> Result<Self, String> {
        let mainloop = Rc::new(RefCell::new(
            Mainloop::new().ok_or("pulseaudio: failed to create main loop")?,
        ));

        let ctx = Rc::new(RefCell::new(
            Context::new(&*mainloop.borrow(), "Volume change")
                .ok_or("pulseaudio: failed to create context")?,
        ));

        // Setup context state change callback
        {
            let mainloop_ref = Rc::clone(&mainloop);
            let ctx_ref = Rc::clone(&ctx);

            ctx.borrow_mut().set_state_callback(Some(Box::new(move || {
                // Unfortunately, we need to bypass the runtime borrow
                // checker here of RefCell here, see
                // https://github.com/jnqnfe/pulse-binding-rust/issues/19
                // for details.
                let state = unsafe { &*ctx_ref.as_ptr() } // Borrow checker workaround
                    .get_state();
                match state {
                    context::State::Ready | context::State::Failed | context::State::Terminated => {
                        unsafe { &mut *mainloop_ref.as_ptr() } // Borrow checker workaround
                            .signal(false);
                    }
                    _ => {}
                }
            })));
        }

        ctx.borrow_mut()
            .connect(None, FlagSet::NOAUTOSPAWN, None)
            .map_err(|err| format!("pulseaudio: failed to connect context: {}", err))?;

        mainloop.borrow_mut().lock();

        if let Err(err) = mainloop.borrow_mut().start() {
            mainloop.borrow_mut().unlock();
            panic!("pulseaudio: failed to start mainloop: {err}");
        }

        // Wait for context to be ready
        loop {
            match ctx.borrow().get_state() {
                State::Ready => {
                    break;
                }
                State::Failed | State::Terminated => {
                    mainloop.borrow_mut().unlock();
                    mainloop.borrow_mut().stop();
                    return Err("pulseaudio: context state failed/terminated unexpectedly".into());
                }
                _ => {
                    mainloop.borrow_mut().wait();
                }
            }
        }
        ctx.borrow_mut().set_state_callback(None);

        mainloop.borrow_mut().unlock();

        Ok(Self { ctx, ml: mainloop })
    }
    fn with_lock<T>(&self, f: impl FnOnce(&PulseAudioThing) -> T) -> T {
        self.ml.borrow_mut().lock();
        let res = f(self);
        let ml_clone = self.ml.clone();
        if let Some(o) = self.ctx.borrow_mut().drain(move || {
            unsafe { (*ml_clone.as_ptr()).signal(false) };
        }) {
            println!("drain");
            while o.get_state() != pulse::operation::State::Done {
                self.ml.borrow_mut().wait();
            }
        }
        self.ml.borrow_mut().unlock();
        res
    }
}

// fn pamixer_cmd(sink_or_source: OptionalSinkOrSource, info: PaMixerSignalInfo) {
//     let mut cmd = vec!["pamixer".to_string()];
//     match sink_or_source.0.as_ref() {
//         OptionalSinkOrSourceDevice::Sink(os) => {
//             if let Some(s) = os {
//                 match match_name_index_sink(s) {
//                     Ok(index) => {
//                         cmd.push(format!("--sink {index}"));
//                     }
//                     Err(e) => {
//                         log::error!("Error getting sink index: {e}");
//                     }
//                 };
//             }
//         }
//         OptionalSinkOrSourceDevice::Source(os) => {
//             if let Some(s) = os {
//                 match match_name_index_source(s) {
//                     Ok(index) => {
//                         cmd.push(format!("--source {index}"));
//                     }
//                     Err(e) => {
//                         log::error!("Error getting sink index: {e}");
//                     }
//                 };
//             } else {
//                 cmd.push("--default-source".to_string());
//             }
//         }
//     };
//     match info {
//         PaMixerSignalInfo::Volume(f) => {
//             cmd.push(format!("--set-volume {}", (f * 100.) as u32));
//         }
//         PaMixerSignalInfo::Mute(m) => cmd.push(
//             match m {
//                 true => "--mute",
//                 false => "--unmute",
//             }
//             .to_string(),
//         ),
//     };
//     log::debug!("execute pamixer cmd");
//     shell_cmd(cmd.join(" "));
//     // gio::spawn_blocking(move || {
//     // });
// }

fn set_sink(pat: Rc<PulseAudioThing>, s: &str, info: VolOrMute) {
    let is_done = Arc::new(RwLock::new(false));
    let _is_done = is_done.clone();
    let pat_clone = pat.clone();

    let cb: Box<dyn FnMut(SinkOrSourceInfo)> = match info {
        VolOrMute::Volume(p) => Box::new(move |si| {
            if let SinkOrSourceInfo::Sink(si) = si {
                log::debug!("start set sink volume");
                let mut cv = si.volume;
                let cv_len = cv.len();
                let v = Volume((p * (Volume::NORMAL.0 as f64)) as u32);
                cv.set(cv_len, v);
                let is_done = _is_done.clone();
                let mut ins = unsafe { (*pat_clone.ctx.as_ptr()).introspect() };
                ins.set_sink_volume_by_index(
                    si.index,
                    &cv,
                    Some(Box::new(move |f| {
                        *is_done.write().unwrap() = true;
                        if !f {
                            log::error!("Fail to set sink volume");
                        } else {
                            println!("success")
                        }
                    })),
                );
            }
        }),
        VolOrMute::Mute(m) => Box::new(move |si| {
            if let SinkOrSourceInfo::Sink(si) = si {
                log::debug!("start set sink mute");
                let is_done = _is_done.clone();
                let mut ins = unsafe { (*pat_clone.ctx.as_ptr()).introspect() };
                ins.set_sink_mute_by_index(
                    si.index,
                    m,
                    Some(Box::new(move |f| {
                        *is_done.write().unwrap() = true;
                        if !f {
                            log::error!("Fail to set sink mute");
                        }
                    })),
                );
            }
        }),
    };
    log::debug!("start with lock");
    let (is_list_end, is_matched) =
        pat.with_lock(move |pat_ref| _match_sink(pat_ref, s.to_string(), cb));
    println!("wait");
    while !*is_list_end.read().unwrap() {
        spin_loop();
    }
    if *is_matched.read().unwrap() {
        while !*is_done.read().unwrap() {}
    }
    println!("done");
}

fn _match_sink(
    pat: &PulseAudioThing,
    s: String,
    mut cb: impl FnMut(SinkOrSourceInfo) + 'static,
) -> (Arc<RwLock<bool>>, Arc<RwLock<bool>>) {
    let is_list_end = Arc::new(RwLock::new(false));
    let _is_list_end = is_list_end.clone();

    let is_matched = Arc::new(RwLock::new(false));
    let _is_matched = is_matched.clone();
    let ins = unsafe { (*pat.ctx.as_ptr()).introspect() };
    ins.get_sink_info_list(move |l| {
        match l {
            libpulse_binding::callbacks::ListResult::Item(si) => {
                *is_matched.write().unwrap() = true;
                let a: &str = si.description.as_ref().unwrap();
                if a.eq(&s) {
                    cb(SinkOrSourceInfo::Sink(si));
                };
            }
            libpulse_binding::callbacks::ListResult::End => *is_list_end.write().unwrap() = true,
            libpulse_binding::callbacks::ListResult::Error => *is_list_end.write().unwrap() = true,
        };
    });

    (_is_list_end, _is_matched)
}

fn set_source(pat: Rc<PulseAudioThing>, s: &str, info: VolOrMute) {
    let is_done = Arc::new(RwLock::new(false));
    let _is_done = is_done.clone();
    let pat_clone = pat.clone();

    let cb: Box<dyn FnMut(SinkOrSourceInfo)> = match info {
        VolOrMute::Volume(p) => Box::new(move |si| {
            if let SinkOrSourceInfo::Source(si) = si {
                log::debug!("start set source volume");
                let mut cv = si.volume;
                let cv_len = cv.len();
                let v = Volume((p * (Volume::NORMAL.0 as f64)) as u32);
                cv.set(cv_len, v);
                let is_done = _is_done.clone();
                let mut ins = unsafe { (*pat_clone.ctx.as_ptr()).introspect() };
                ins.set_source_volume_by_index(
                    si.index,
                    &cv,
                    Some(Box::new(move |f| {
                        *is_done.write().unwrap() = true;
                        if !f {
                            log::error!("Fail to set source volume");
                        } else {
                            println!("success")
                        }
                    })),
                );
            }
        }),
        VolOrMute::Mute(m) => Box::new(move |si| {
            if let SinkOrSourceInfo::Source(si) = si {
                log::debug!("start set source mute");
                let is_done = _is_done.clone();
                let mut ins = unsafe { (*pat_clone.ctx.as_ptr()).introspect() };
                ins.set_source_mute_by_index(
                    si.index,
                    m,
                    Some(Box::new(move |f| {
                        *is_done.write().unwrap() = true;
                        if !f {
                            log::error!("Fail to set source mute");
                        }
                    })),
                );
            }
        }),
    };
    log::debug!("start with lock");
    let (is_list_end, is_matched) =
        pat.with_lock(move |pat_ref| _match_source(pat_ref, s.to_string(), cb));
    println!("wait");
    while !*is_list_end.read().unwrap() {
        spin_loop();
    }
    if *is_matched.read().unwrap() {
        while !*is_done.read().unwrap() {}
    }
    println!("done");
}

fn _match_source(
    pat: &PulseAudioThing,
    s: String,
    mut cb: impl FnMut(SinkOrSourceInfo) + 'static,
) -> (Arc<RwLock<bool>>, Arc<RwLock<bool>>) {
    let is_list_end = Arc::new(RwLock::new(false));
    let _is_list_end = is_list_end.clone();

    let is_matched = Arc::new(RwLock::new(false));
    let _is_matched = is_matched.clone();

    let ins = unsafe { (*pat.ctx.as_ptr()).introspect() };
    ins.get_source_info_list(move |l| {
        match l {
            libpulse_binding::callbacks::ListResult::Item(si) => {
                *is_matched.write().unwrap() = true;
                let a: &str = si.description.as_ref().unwrap();
                if a.eq(&s) {
                    cb(SinkOrSourceInfo::Source(si));
                };
            }
            libpulse_binding::callbacks::ListResult::End => *is_list_end.write().unwrap() = true,
            libpulse_binding::callbacks::ListResult::Error => *is_list_end.write().unwrap() = true,
        };
    });
    (_is_list_end, _is_matched)
}
