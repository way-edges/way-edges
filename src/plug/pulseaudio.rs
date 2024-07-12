use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    ops::DerefMut,
    rc::{Rc, Weak},
    sync::{
        atomic::{AtomicBool, AtomicPtr, Ordering},
        Arc, OnceLock, RwLock,
    },
    thread,
};

use async_channel::Sender;
use gio::glib::{clone::Downgrade, subclass::shared::RefCounted};
use libpulse_binding::{
    self as pulse,
    callbacks::ListResult,
    context::{
        introspect::{Introspector, SinkInfo, SourceInfo},
        subscribe::InterestMaskSet,
    },
    def::Retval,
    mainloop::standard::IterateResult,
    volume::{ChannelVolumes, Volume},
};

use gtk::glib;
use pulse::{
    context::{Context, FlagSet},
    mainloop::standard::Mainloop,
};

pub type PaCallback = dyn FnMut(VInfo, InterestMaskSet);
pub type PaErrCallback = dyn FnMut(String);

struct PA {
    count: i32,
    // sink_cbs: Vec<Rc<RefCell<PaCallback>>>,
    // source_cbs: Vec<Rc<RefCell<PaCallback>>>,
    // on_error_cbs: Vec<Box<PaErrCallback>>,
    sink_cbs: HashMap<i32, Rc<RefCell<PaCallback>>>,
    source_cbs: HashMap<i32, Rc<RefCell<PaCallback>>>,
    on_error_cbs: Vec<Box<PaErrCallback>>,
}

impl PA {
    fn call(&mut self, sink_or_source: InterestMaskSet) {
        if sink_or_source.contains(InterestMaskSet::SINK) {
            log::debug!("call sink cb");
            self.sink_cbs.iter_mut().for_each(|(_, f)| {
                let mut f = f.borrow_mut();
                f(get_global_pa_sink().unwrap(), sink_or_source);
            });
        } else if sink_or_source.contains(InterestMaskSet::SOURCE) {
            log::debug!("call source cb");
            self.source_cbs.iter_mut().for_each(|(_, f)| {
                let mut f = f.borrow_mut();
                f(get_global_pa_source().unwrap(), sink_or_source);
            });
        };
    }
    fn add_cb(
        &mut self,
        cb: Box<PaCallback>,
        error_cb: Option<impl FnMut(String) + 'static>,
        sink_or_source: InterestMaskSet,
    ) -> i32 {
        let cb = Rc::new(RefCell::new(cb));
        let key = self.count;
        if sink_or_source.contains(InterestMaskSet::SINK) {
            log::debug!("add sink cb");
            cb.borrow_mut()(get_global_pa_sink().unwrap(), InterestMaskSet::SINK);
            self.sink_cbs.insert(key, cb.clone());
        }
        if sink_or_source.contains(InterestMaskSet::SOURCE) {
            log::debug!("add source cb");
            cb.borrow_mut()(get_global_pa_source().unwrap(), InterestMaskSet::SOURCE);
            self.source_cbs.insert(key, cb.clone());
        }
        if let Some(error_cb) = error_cb {
            self.on_error_cbs.push(Box::new(error_cb));
        };
        self.count += 1;
        key
    }
    fn remove_cb(&mut self, key: i32) {
        self.sink_cbs.remove_entry(&key);
        self.source_cbs.remove_entry(&key);
    }
    fn error(self, e: String) {
        log::error!("Pulseaudio error(quit mainloop because of this): {e}");
        self.on_error_cbs.into_iter().for_each(|mut f| f(e.clone()));
    }
}

static IS_PA_INITIALIZED: AtomicBool = AtomicBool::new(false);
// static mut PA_CONTEXT: Option<PA> = None;
static PA_CONTEXT: AtomicPtr<PA> = AtomicPtr::new(std::ptr::null_mut());
fn init_pa() {
    IS_PA_INITIALIZED.store(true, Ordering::Release);
    PA_CONTEXT.store(
        Box::into_raw(Box::new(PA {
            count: 0,
            // sink_cbs: vec![],
            // source_cbs: vec![],
            sink_cbs: HashMap::new(),
            source_cbs: HashMap::new(),
            on_error_cbs: vec![],
        })),
        Ordering::Release,
    );
}
fn on_pa_error(e: String) {
    unsafe {
        let pa_ptr = PA_CONTEXT.swap(std::ptr::null_mut(), Ordering::Release);
        if pa_ptr.is_null() {
            return;
        }
        let a = pa_ptr.read();
        a.error(e);
    }
}
fn is_pa_inited() -> bool {
    IS_PA_INITIALIZED.load(Ordering::Acquire)
}
fn is_pa_empty() -> bool {
    PA_CONTEXT.load(Ordering::Acquire).is_null()
}
fn call_pa(sink_or_source: InterestMaskSet) {
    unsafe {
        PA_CONTEXT
            .load(Ordering::Acquire)
            .as_mut()
            .unwrap()
            .call(sink_or_source);
    }
}
fn add_cb(
    cb: Box<PaCallback>,
    error_cb: Option<impl FnMut(String) + 'static>,
    sink_or_source: InterestMaskSet,
) -> i32 {
    unsafe {
        PA_CONTEXT
            .load(Ordering::Acquire)
            .as_mut()
            .unwrap()
            .add_cb(cb, error_cb, sink_or_source)
    }
}
fn rm_cb(key: i32) {
    unsafe {
        PA_CONTEXT
            .load(Ordering::Acquire)
            .as_mut()
            .unwrap()
            .remove_cb(key);
    }
}

pub fn try_init_pulseaudio() -> Result<(), String> {
    if !is_pa_inited() {
        let sr = init_mainloop()?;
        init_pa();
        glib::spawn_future_local(async move {
            log::debug!("start pulseaudio signal receiver on glib main thread");
            loop {
                if let Ok(r) = sr.recv().await {
                    log::debug!("recv pulseaudio signal: {r:#?}");
                    match r {
                        Ok(r) => {
                            call_pa(r);
                        }
                        Err(e) => {
                            on_pa_error(e);
                            break;
                        }
                    }
                } else {
                    on_pa_error(
                        "pulseaudio mainloops seems closed(communication channel closed)"
                            .to_string(),
                    );
                    break;
                }
            }
        });
    } else if is_pa_empty() {
        return Err(
            "pulseaudio mainloops seems inited before closed due to some error".to_string(),
        );
    }
    if get_global_pa_sink().is_none() || get_global_pa_source().is_none() {
        return Err("pulseaudio sink or source is not available".to_string());
    }
    Ok(())
}

pub fn register_callback(
    cb: impl FnMut(VInfo, InterestMaskSet) + 'static,
    error_cb: Option<impl FnMut(String) + 'static>,
    sink_or_source: InterestMaskSet,
) -> Result<i32, String> {
    try_init_pulseaudio()?;
    Ok(add_cb(Box::new(cb), error_cb, sink_or_source))
}

pub fn unregister_callback(key: i32) {
    if !is_pa_empty() {
        rm_cb(key);
    }
}

fn get_avg_volume(cv: ChannelVolumes) -> f64 {
    cv.avg().0 as f64 / Volume::NORMAL.0 as f64
}

fn iter_loop(ml: &mut Mainloop) -> Result<(), String> {
    match ml.iterate(true) {
        IterateResult::Quit(r) => Err(format!("mainloop quit: with status: {r:?}")),
        IterateResult::Err(e) => Err(format!("mainloop iterate Error: {e}")),
        IterateResult::Success(_) => Ok(()),
    }
}

#[derive(Debug, Clone)]
pub struct VInfo {
    pub vol: f64,
    pub is_muted: bool,
}

static GLOBAL_PA_SINK: OnceLock<Arc<RwLock<Option<VInfo>>>> = OnceLock::new();
fn get_global_pa_sink_ptr() -> &'static Arc<RwLock<Option<VInfo>>> {
    GLOBAL_PA_SINK.get_or_init(|| Arc::new(RwLock::new(None)))
}
fn get_global_pa_sink() -> Option<VInfo> {
    get_global_pa_sink_ptr().read().unwrap().clone()
}
fn set_global_pa_sink(vol: f64, is_muted: bool) {
    *get_global_pa_sink_ptr().write().unwrap() = Some(VInfo { vol, is_muted });
}
static GLOBAL_PA_SOURCE: OnceLock<Arc<RwLock<Option<VInfo>>>> = OnceLock::new();
fn get_global_pa_source_ptr() -> &'static Arc<RwLock<Option<VInfo>>> {
    GLOBAL_PA_SOURCE.get_or_init(|| Arc::new(RwLock::new(None)))
}
fn get_global_pa_source() -> Option<VInfo> {
    get_global_pa_source_ptr().read().unwrap().clone()
}
fn set_global_pa_source(vol: f64, is_muted: bool) {
    *get_global_pa_source_ptr().write().unwrap() = Some(VInfo { vol, is_muted });
}

type Signal = Result<InterestMaskSet, String>;

fn init_mainloop() -> Result<async_channel::Receiver<Signal>, String> {
    // subscribe
    let (ss, sr) = async_channel::bounded::<Result<InterestMaskSet, String>>(1);

    fn close_mainloop(m: &std::rc::Weak<RefCell<Mainloop>>) {
        if let Some(m) = m.upgrade() {
            unsafe {
                let a = RefCell::as_ptr(&m).as_mut().unwrap();
                a.quit(Retval(1));
            }
        }
    }
    fn process_sink(ls: ListResult<&SinkInfo>, ss: &Sender<Signal>, mc: &Weak<RefCell<Mainloop>>) {
        match ls {
            pulse::callbacks::ListResult::Item(res) => {
                let avg = get_avg_volume(res.volume);
                set_global_pa_sink(avg, res.mute);
                if ss.force_send(Ok(InterestMaskSet::SINK)).is_err() {
                    close_mainloop(mc);
                }
            }
            pulse::callbacks::ListResult::End => {}
            pulse::callbacks::ListResult::Error => {
                close_mainloop(mc);
                ss.force_send(Err("Error getting sink info".to_string()))
                    .ok();
            }
        };
    }
    fn process_source(
        ls: ListResult<&SourceInfo>,
        ss: &Sender<Signal>,
        mc: &Weak<RefCell<Mainloop>>,
    ) {
        match ls {
            pulse::callbacks::ListResult::Item(res) => {
                let avg = get_avg_volume(res.volume);
                set_global_pa_source(avg, res.mute);
                if ss.force_send(Ok(InterestMaskSet::SOURCE)).is_err() {
                    close_mainloop(mc);
                }
            }
            pulse::callbacks::ListResult::End => {}
            pulse::callbacks::ListResult::Error => {
                close_mainloop(mc);
                ss.force_send(Err("Error getting source info".to_string()))
                    .ok();
            }
        };
    }
    let update_sink_by_index = {
        let ss_clone = ss.clone();
        move |ins: Introspector, index: u32, mc: Weak<RefCell<Mainloop>>| {
            let ss = ss_clone.clone();
            ins.get_sink_info_by_index(index, move |ls| {
                process_sink(ls, &ss, &mc);
            });
        }
    };
    let update_source_by_index = {
        let ss_clone = ss.clone();
        move |ins: Introspector, index: u32, mc: Weak<RefCell<Mainloop>>| {
            let ss = ss_clone.clone();
            ins.get_source_info_by_index(index, move |ls| {
                process_source(ls, &ss, &mc);
            });
        }
    };

    // atual logic
    {
        // init
        let (ps, pr) = async_channel::bounded::<Result<(), String>>(1);
        thread::spawn(move || {
            let ss_clone = ss.clone();
            let res = move || -> Result<(Rc<RefCell<Mainloop>>, Rc<RefCell<Context>>), String> {
                let mainloop = Mainloop::new().ok_or("Failed to create mainloop")?;
                let mut context =
                    Context::new(&mainloop, "Volume Monitor").ok_or("Failed to create context")?;

                context
                    .connect(None, FlagSet::NOAUTOSPAWN, None)
                    .map_err(|e| format!("Failed to connect context: {e}"))?;

                let context = Rc::new(RefCell::new(context));
                let mainloop = Rc::new(RefCell::new(mainloop));

                let ready = Rc::new(Cell::new(false));
                let ready_clone = ready.clone();
                let context_clone = context.clone();
                let mainloop_clone = Rc::downgrade(&mainloop);
                {
                    let ss = ss_clone.clone();
                    context
                        .borrow_mut()
                        .set_state_callback(Some(Box::new(move || {
                            let state = context_clone.borrow().get_state();
                            match state {
                                pulse::context::State::Unconnected => {
                                    close_mainloop(&mainloop_clone);
                                    ss.force_send(Err("PulseAudio callback error".to_string()))
                                        .unwrap();
                                }
                                pulse::context::State::Ready => {
                                    ready_clone.set(true);
                                }
                                _ => {}
                            }
                        })));
                }

                while !ready.get() {
                    iter_loop(mainloop.borrow_mut().deref_mut())?;
                }

                log::debug!("start subscribe pulseaudio sink and source");
                {
                    let mut ctx = context.borrow_mut();
                    {
                        let res = Rc::new(Cell::new(None));
                        let res_clone = res.clone();
                        ctx.subscribe(InterestMaskSet::SINK | InterestMaskSet::SOURCE, move |s| {
                            res_clone.set(Some(s));
                        });
                        while res.get().is_none() {
                            iter_loop(mainloop.borrow_mut().deref_mut())?;
                        }
                        let res = res.get().unwrap();
                        if !res {
                            panic!("fail to subscribe pulseaudio");
                        }
                    };
                    {
                        let context_clone = context.clone();
                        let mainloop_clone = Rc::downgrade(&mainloop);
                        ctx.set_subscribe_callback(Some(Box::new(
                            move |facility, operation, index| {
                                log::debug!(
                                    "{facility:?} event occurred: {:?}, index: {}",
                                    operation,
                                    index
                                );
                                let ins = context_clone.borrow().introspect();
                                let mc = mainloop_clone.clone();
                                match facility.unwrap() {
                                    pulse::context::subscribe::Facility::Sink => {
                                        update_sink_by_index(ins, index, mc);
                                    }
                                    pulse::context::subscribe::Facility::Source => {
                                        update_source_by_index(ins, index, mc);
                                    }
                                    _ => {}
                                };
                            },
                        )));
                    }
                };
                Ok((mainloop, context))
            }();
            let (mainloop, context) = match res {
                Ok(r) => r,
                Err(e) => {
                    ps.try_send(Err(e)).ok();
                    return;
                }
            };
            // first
            // let data_res = Rc::new(Cell::new(None::<Result<(), &str>>));
            let data_res = Rc::new(RefCell::new(Ok((false, false))));
            let data_res_clone = data_res.clone();
            let ins = Rc::new(RefCell::new(context.borrow().introspect()));
            let ins_clone = ins.clone();
            let mainloop_clone = mainloop.downgrade();
            let ss_clone = ss.clone();
            ins.borrow().get_server_info(move |s| {
                let res = s
                    .default_sink_name
                    .as_ref()
                    .ok_or("default sink not found")
                    .and_then(|sink_name| {
                        s.default_source_name
                            .as_ref()
                            .ok_or("default source not found")
                            .map(|source_name| (sink_name, source_name))
                    });
                let (sink_name, source_name) = match res {
                    Ok(r) => (r.0, r.1),
                    Err(e) => {
                        *data_res_clone.borrow_mut() = Err(e.to_string());
                        return;
                    }
                };

                let ss = ss_clone.clone();
                let mainloop = mainloop_clone.clone();
                let data_res = data_res_clone.clone();
                ins_clone
                    .borrow()
                    .get_sink_info_by_name(sink_name, move |ls| {
                        process_sink(ls, &ss, &mainloop);
                        if let Ok(e) = data_res.borrow_mut().as_mut() {
                            e.0 = true;
                        }
                    });

                let ss = ss_clone.clone();
                let mainloop = mainloop_clone.clone();
                let data_res = data_res_clone.clone();
                ins_clone
                    .borrow()
                    .get_source_info_by_name(source_name, move |ls| {
                        process_source(ls, &ss, &mainloop);
                        if let Ok(e) = data_res.borrow_mut().as_mut() {
                            e.1 = true;
                        }
                    });
            });
            while {
                let temp = data_res.borrow();
                if let Ok(r) = temp.as_ref() {
                    !r.0 || !r.1
                } else {
                    false
                }
            } {
                if let Err(e) = iter_loop(mainloop.borrow_mut().deref_mut()) {
                    *data_res.borrow_mut() = Err(e);
                };
            }

            unsafe {
                let a = data_res.into_raw().read();
                if let Err(e) = a.into_inner() {
                    ps.send_blocking(Err(e)).ok();
                    return;
                }
            }

            if ps.send_blocking(Ok(())).is_err() {
                mainloop.borrow_mut().quit(Retval(1));
                return;
            };

            log::info!("start running pulseaudio mainloop");

            if let Err(e) = mainloop.borrow_mut().run() {
                ss.force_send(Err(format!("Error running mainloop: {e:?}")))
                    .ok();
            };
            log::info!("quit pulseaudio mainloop");
        });
        pr.recv_blocking().unwrap()?;
    };
    Ok(sr)
}
