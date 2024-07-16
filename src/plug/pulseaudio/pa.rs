use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    hint::spin_loop,
    ops::DerefMut,
    rc::{Rc, Weak},
    sync::{
        atomic::{AtomicPtr, Ordering},
        Arc, OnceLock, RwLock,
    },
    thread::{self},
};

use gio::glib::{clone::Downgrade, subclass::shared::RefCounted};
use gtk::glib;
use libpulse_binding::{
    self as pulse,
    callbacks::ListResult,
    context::{
        introspect::{Introspector, SinkInfo, SourceInfo},
        subscribe::InterestMaskSet,
        Context, FlagSet,
    },
    def::Retval,
    mainloop::standard::{IterateResult, Mainloop},
    volume::{ChannelVolumes, Volume},
};

#[derive(Debug, Clone)]
pub struct VInfo {
    pub vol: f64,
    pub is_muted: bool,
}

static DEFAULT_SINK: AtomicPtr<String> = AtomicPtr::new(std::ptr::null_mut());
static DEFAULT_SOURCE: AtomicPtr<String> = AtomicPtr::new(std::ptr::null_mut());
pub fn get_default_sink() -> Option<&'static String> {
    unsafe { DEFAULT_SINK.load(Ordering::Acquire).as_ref() }
}
fn set_default_sink(s: String) {
    DEFAULT_SINK.store(Box::into_raw(Box::new(s)), Ordering::Release)
}
pub fn get_default_source() -> Option<&'static String> {
    unsafe { DEFAULT_SOURCE.load(Ordering::Acquire).as_ref() }
}
fn set_default_source(s: String) {
    DEFAULT_SOURCE.store(Box::into_raw(Box::new(s)), Ordering::Release)
}

type VInfoMap = HashMap<String, VInfo>;
static VINFOS: OnceLock<Arc<RwLock<(VInfoMap, VInfoMap)>>> = OnceLock::new();

fn get_vinfos() -> &'static Arc<RwLock<(VInfoMap, VInfoMap)>> {
    VINFOS.get_or_init(|| Arc::new(RwLock::new((HashMap::new(), HashMap::new()))))
}

pub fn get_sink_vol_by_name(n: &str) -> Option<VInfo> {
    log::debug!("get_sink_vol_by_name {}", n);
    let a = get_vinfos().read().unwrap().0.get(n).cloned();
    log::debug!("get_sink_vol_by_name done {}", n);
    a
}
pub fn get_source_vol_by_name(n: &str) -> Option<VInfo> {
    log::debug!("get_source_vol_by_name {}", n);
    let a = get_vinfos().read().unwrap().1.get(n).cloned();
    log::debug!("get_source_vol_by_name done {}", n);
    a
}
pub fn set_sink_vol_by_name(n: String, v: VInfo) {
    log::debug!("set_sink_vol_by_name {}", n);
    get_vinfos().write().unwrap().0.insert(n, v);
    log::debug!("set_sink_vol_by_name done");
}
pub fn set_source_vol_by_name(n: String, v: VInfo) {
    log::debug!("set_source_vol_by_name {}", n);
    get_vinfos().write().unwrap().1.insert(n, v);
    log::debug!("set_source_vol_by_name done");
}

#[derive(Debug, Clone)]
pub enum SinkOrSourceIndex {
    Sink(u32),
    Source(u32),
}

pub enum SinkOrSourceInfo<'a, 'b: 'a> {
    Sink(&'b SinkInfo<'a>),
    Source(&'b SourceInfo<'a>),
}

type ReloadCallback = Box<dyn FnOnce(SinkOrSourceInfo)>;

// static INTORSPECTOR: AtomicPtr<Context> = AtomicPtr::new(std::ptr::null_mut());
static CONTEXT: AtomicPtr<Context> = AtomicPtr::new(std::ptr::null_mut());
fn set_context(i: Context) {
    let boxed = Box::new(i);
    let raw_ptr = Box::into_raw(boxed);
    CONTEXT.store(raw_ptr, Ordering::Release);
    // INTORSPECTOR.store(Box::into_raw(Box::new(i)), Ordering::Release)
}
fn get_context() -> &'static Context {
    let a = CONTEXT.load(Ordering::Acquire);
    unsafe { &*a }
}
fn get_context_mut() -> &'static mut Context {
    let a = CONTEXT.load(Ordering::Acquire);
    unsafe { &mut *a }
}
pub fn get_introspector() -> Introspector {
    get_context().introspect()
}
pub fn _reload_device_vinfo(
    sosi: SinkOrSourceIndex,
    mut f: Option<ReloadCallback>,
) -> Rc<Cell<bool>> {
    log::debug!("start reload device vinfo");
    let ins = get_introspector();
    let is_done = Rc::new(Cell::new(false));

    log::debug!("start match device");
    let _is_done = is_done.clone();
    match sosi {
        SinkOrSourceIndex::Sink(i) => {
            let cb = move |ls: ListResult<&SinkInfo>| {
                log::debug!("start process sink");
                if let Some(s) = process_sink(ls) {
                    if let Some(f) = f.take() {
                        let a = SinkOrSourceInfo::Sink(s);
                        f(a);
                    };
                };
                _is_done.set(true);
            };
            ins.get_sink_info_by_index(i, cb);
        }
        SinkOrSourceIndex::Source(i) => {
            let cb = move |ls: ListResult<&SourceInfo>| {
                log::debug!("start process source");
                if let Some(s) = process_source(ls) {
                    if let Some(f) = f.take() {
                        f(SinkOrSourceInfo::Source(s));
                    };
                };
                _is_done.set(true);
            };
            ins.get_source_info_by_index(i, cb);
        }
    }

    is_done
}
pub fn reload_device_vinfo(sosi: SinkOrSourceIndex, f: Option<ReloadCallback>) {
    _reload_device_vinfo(sosi, f);
}
pub fn reload_device_vinfo_blocking(sosi: SinkOrSourceIndex, f: Option<ReloadCallback>) {
    let is_done = _reload_device_vinfo(sosi, f);
    while !is_done.get() {
        spin_loop();
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SinkOrSource {
    Sink(String),
    Source(String),
}

pub type Signal = Result<SinkOrSource, String>;

fn process_sink<'a>(ls: ListResult<&'a SinkInfo>) -> Option<&'a SinkInfo<'a>> {
    match ls {
        pulse::callbacks::ListResult::Item(res) => {
            let avg = get_avg_volume(res.volume);
            set_sink_vol_by_name(
                res.description.clone().unwrap().to_string(),
                VInfo {
                    vol: avg,
                    is_muted: res.mute,
                },
            );
            return Some(res);
        }
        pulse::callbacks::ListResult::End => {}
        pulse::callbacks::ListResult::Error => {
            log::error!("Error getting sink info");
        }
    };
    None
}

fn process_source<'a>(ls: ListResult<&'a SourceInfo>) -> Option<&'a SourceInfo<'a>> {
    match ls {
        pulse::callbacks::ListResult::Item(res) => {
            let avg = get_avg_volume(res.volume);
            set_source_vol_by_name(
                res.description.clone().unwrap().to_string(),
                VInfo {
                    vol: avg,
                    is_muted: res.mute,
                },
            );
            return Some(res);
        }
        pulse::callbacks::ListResult::End => {}
        pulse::callbacks::ListResult::Error => {
            log::error!("Error getting source info");
        }
    };
    None
}

pub fn init_mainloop() -> Result<async_channel::Receiver<Signal>, String> {
    // subscribe
    let (ss, sr) = async_channel::bounded::<Signal>(1);

    fn close_mainloop(m: &std::rc::Weak<RefCell<Mainloop>>) {
        if let Some(m) = m.upgrade() {
            unsafe {
                let a = RefCell::as_ptr(&m).as_mut().unwrap();
                a.quit(Retval(1));
            }
        }
    }
    let update_sink_by_index = {
        let ss_clone = ss.clone();
        // move |ins: Introspector, index: u32, mc: Weak<RefCell<Mainloop>>| {
        move |index: u32, mc: Weak<RefCell<Mainloop>>| {
            let ss = ss_clone.clone();
            reload_device_vinfo(
                SinkOrSourceIndex::Sink(index),
                Some(Box::new(move |res| {
                    log::debug!("run reload sink vinfo cb");
                    if let SinkOrSourceInfo::Sink(res) = res {
                        if ss
                            .force_send(Ok(SinkOrSource::Sink(
                                res.description.clone().unwrap().to_string(),
                            )))
                            .is_err()
                        {
                            log::error!(
                                "Error sending sink change signal(receiver closed), close mainloop"
                            );
                            close_mainloop(&mc);
                        }
                    }
                    log::debug!("run reload sink vinfo done");
                })),
            );
        }
    };
    let update_source_by_index = {
        let ss_clone = ss.clone();
        // move |ins: Introspector, index: u32, mc: Weak<RefCell<Mainloop>>| {
        move |index: u32, mc: Weak<RefCell<Mainloop>>| {
            let ss = ss_clone.clone();
            reload_device_vinfo(
                SinkOrSourceIndex::Source(index),
                Some(Box::new(move |res| {
                    if let SinkOrSourceInfo::Source(res) = res {
                        if ss
                            .force_send(Ok(SinkOrSource::Source(
                                res.description.clone().unwrap().to_string(),
                            )))
                            .is_err()
                        {
                            log::error!(
                            "Error sending source change signal(receiver closed), close mainloop"
                        );
                            close_mainloop(&mc);
                        }
                    }
                })),
            );
        }
    };

    // atual logic
    {
        // init
        let (ps, pr) = async_channel::bounded::<Result<(), String>>(1);
        thread::spawn(move || {
            let ss_clone = ss.clone();
            // let res = move || -> Result<(Rc<RefCell<Mainloop>>, Rc<RefCell<Context>>), String> {
            let res = move || -> Result<Rc<RefCell<Mainloop>>, String> {
                let mainloop = Mainloop::new().ok_or("Failed to create mainloop")?;
                let mut context =
                    Context::new(&mainloop, "Volume Monitor").ok_or("Failed to create context")?;

                context
                    .connect(None, FlagSet::NOAUTOSPAWN, None)
                    .map_err(|e| format!("Failed to connect context: {e}"))?;
                set_context(context);

                // let context = Rc::new(RefCell::new(context));
                let mainloop = Rc::new(RefCell::new(mainloop));

                let ready = Rc::new(Cell::new(false));
                let ready_clone = ready.clone();
                // let context_clone = context.clone();
                let mainloop_clone = Rc::downgrade(&mainloop);
                {
                    let ss = ss_clone.clone();
                    // context
                    //     .borrow_mut()
                    get_context_mut().set_state_callback(Some(Box::new(move || {
                        // let state = context_clone.borrow().get_state();
                        let state = get_context().get_state();
                        match state {
                            pulse::context::State::Unconnected => {
                                close_mainloop(&mainloop_clone);
                                ss.force_send(Err("PulseAudio callback error".to_string()))
                                    .unwrap();
                            }
                            pulse::context::State::Ready => {
                                ready_clone.set(true);
                            }
                            _ => {
                                log::warn!("Unknow state");
                            }
                        }
                    })));
                }

                while !ready.get() {
                    iter_loop(mainloop.borrow_mut().deref_mut())?;
                }

                log::debug!("start subscribe pulseaudio sink and source");
                {
                    // let mut ctx = context.borrow_mut();
                    {
                        let res = Rc::new(Cell::new(None));
                        let res_clone = res.clone();
                        // ctx.subscribe(InterestMaskSet::SINK | InterestMaskSet::SOURCE, move |s| {
                        get_context_mut().subscribe(
                            InterestMaskSet::SINK | InterestMaskSet::SOURCE,
                            move |s| {
                                res_clone.set(Some(s));
                            },
                        );
                        while res.get().is_none() {
                            iter_loop(mainloop.borrow_mut().deref_mut())?;
                        }
                        let res = res.get().unwrap();
                        if !res {
                            panic!("fail to subscribe pulseaudio");
                        }
                    };
                    {
                        // let context_clone = context.clone();
                        let mainloop_clone = Rc::downgrade(&mainloop);
                        // ctx.set_subscribe_callback(Some(Box::new(
                        get_context_mut().set_subscribe_callback(Some(Box::new(
                            move |facility, operation, index| {
                                log::debug!(
                                    "{facility:?} event occurred: {:?}, index: {}",
                                    operation,
                                    index
                                );
                                // let ins = context_clone.borrow().introspect();
                                let mc = mainloop_clone.clone();
                                match facility.unwrap() {
                                    pulse::context::subscribe::Facility::Sink => {
                                        // update_sink_by_index(ins, index, mc);
                                        update_sink_by_index(index, mc);
                                    }
                                    pulse::context::subscribe::Facility::Source => {
                                        // update_source_by_index(ins, index, mc);
                                        update_source_by_index(index, mc);
                                    }
                                    _ => {}
                                };
                            },
                        )));
                    }
                };
                // Ok((mainloop, context))
                Ok(mainloop)
            }();
            // let (mainloop, context) = match res {
            let mainloop = match res {
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
            // let ins = Rc::new(RefCell::new(context.borrow().introspect()));
            let ins = Rc::new(RefCell::new(get_context().introspect()));
            let ins_clone = ins.clone();
            let mainloop_clone = mainloop.downgrade();
            let ss_clone = ss.clone();
            log::debug!("Getting default sink and source info");
            ins.borrow().get_server_info(move |s| {
                let ( sink_name, source_name ) = (s
                    .default_sink_name.as_ref(), s.default_source_name.as_ref());

                if let Some(sink_name) = sink_name {
                    ins_clone
                        .borrow()
                        .get_sink_info_by_name(
                            sink_name,
                            glib::clone!(@strong ss_clone, @strong mainloop_clone, @strong data_res_clone => move |ls| {
                                if let ListResult::Item(s) = ls {
                                    let desc = s.description.clone().unwrap().to_string();
                                    log::debug!("Set default sink: {desc}");
                                    set_default_sink(desc);
                                }
                                process_sink(ls);
                                data_res_clone.borrow_mut().as_mut().unwrap().0 = true;
                            }));
                }else {
                    log::warn!("Did not get default sink device");
                    data_res_clone.borrow_mut().as_mut().unwrap().0 = true;
                }

                if let Some(source_name) = source_name{
                    ins_clone
                        .borrow()
                        .get_source_info_by_name(
                            source_name,
                            glib::clone!(@strong ss_clone, @strong mainloop_clone, @strong data_res_clone => move |ls| {
                                if let ListResult::Item(s) = ls {
                                    let desc = s.description.clone().unwrap().to_string();
                                    log::debug!("Set default source: {desc}");
                                    set_default_source(desc);
                                }
                                process_source(ls);
                                data_res_clone.borrow_mut().as_mut().unwrap().1 = true;
                            }));
                }else {
                    log::warn!("Did not get default source device");
                    data_res_clone.borrow_mut().as_mut().unwrap().1 = true;
                }
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

            // loop {
            //     if let Err(e) = iter_loop(mainloop.borrow_mut().deref_mut()) {
            //         ss.force_send(Err(format!("Error running mainloop: {e:?}")))
            //             .ok();
            //         break;
            //     }
            // }
            // log::info!("quit pulseaudio mainloop");
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
