use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    ops::DerefMut,
    rc::{Rc, Weak},
    sync::{
        atomic::{AtomicPtr, Ordering},
        Arc, OnceLock, RwLock,
    },
    thread::{self},
};

use async_channel::Sender;
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
    get_vinfos().read().unwrap().0.get(n).cloned()
}
pub fn get_source_vol_by_name(n: &str) -> Option<VInfo> {
    get_vinfos().read().unwrap().1.get(n).cloned()
}
pub fn set_sink_vol_by_name(n: String, v: VInfo) {
    get_vinfos().write().unwrap().0.insert(n, v);
}
pub fn set_source_vol_by_name(n: String, v: VInfo) {
    get_vinfos().write().unwrap().1.insert(n, v);
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SinkOrSource {
    Sink(String),
    Source(String),
}

pub type Signal = Result<SinkOrSource, String>;

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
    fn process_sink(ls: ListResult<&SinkInfo>, ss: &Sender<Signal>, mc: &Weak<RefCell<Mainloop>>) {
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
                if ss
                    .force_send(Ok(SinkOrSource::Sink(
                        res.description.clone().unwrap().to_string(),
                    )))
                    .is_err()
                {
                    log::error!(
                        "Error sending sink change signal(receiver closed), close mainloop"
                    );
                    close_mainloop(mc);
                }
            }
            pulse::callbacks::ListResult::End => {}
            pulse::callbacks::ListResult::Error => {
                log::error!("Error getting sink info");
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

                // if avg == 1. {
                //     println!("!!!!!!!!!!!!!!Source: {:#?}", res);
                //     println!("!!!!!!!!!!!!!!AVG Source: {}, {}", avg, res.volume.avg());
                // }

                set_source_vol_by_name(
                    res.description.clone().unwrap().to_string(),
                    VInfo {
                        vol: avg,
                        is_muted: res.mute,
                    },
                );
                if ss
                    .force_send(Ok(SinkOrSource::Source(
                        res.description.clone().unwrap().to_string(),
                    )))
                    .is_err()
                {
                    log::error!(
                        "Error sending sink change signal(receiver closed), close mainloop"
                    );
                    close_mainloop(mc);
                }
            }
            pulse::callbacks::ListResult::End => {}
            pulse::callbacks::ListResult::Error => {
                log::error!("Error getting source info");
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
            log::debug!("Getting default sink and source info");
            ins.borrow().get_server_info(move |s| {
                let ( sink_name, source_name ) = (s
                    .default_sink_name.as_ref(), s.default_source_name.as_ref());
                // let res = s
                //     .default_sink_name
                //     .as_ref()
                //     .ok_or("default sink not found")
                //     .and_then(|sink_name| {
                //         s.default_source_name
                //             .as_ref()
                //             .ok_or("default source not found")
                //             .map(|source_name| (sink_name, source_name))
                //     });
                // let (sink_name, source_name) = match res {
                //     Ok(r) => (r.0, r.1),
                //     Err(e) => {
                //         *data_res_clone.borrow_mut() = Err(e.to_string());
                //         return;
                //     }
                // };

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
                                process_sink(ls, &ss_clone, &mainloop_clone);
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
                                process_source(ls, &ss_clone, &mainloop_clone);
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
