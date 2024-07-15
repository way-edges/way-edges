mod pa;
mod pamixer;

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, AtomicPtr, Ordering},
        Arc,
    },
};

use gtk::glib;
use pa::{
    get_default_sink, get_default_source, get_sink_vol_by_name, get_source_vol_by_name,
    reload_device_vinfo_blocking, SinkOrSource, SinkOrSourceIndex, VInfo,
};
use pamixer::{
    init_pamixser_thread, match_name_index_sink, match_name_index_source, send_pamixer_signal,
    PaMixerSignalInfo,
};

pub type PaCallback = dyn FnMut(&VInfo);
// pub type PaErrCallback = dyn FnMut(String);

struct PA {
    count: i32,
    sink_cbs: HashMap<i32, (String, Rc<RefCell<PaCallback>>)>,
    source_cbs: HashMap<i32, (String, Rc<RefCell<PaCallback>>)>,

    device_map: HashMap<SinkOrSource, HashMap<i32, ()>>,
    // on_error_cbs: Vec<Box<PaErrCallback>>,
}

impl PA {
    fn new() -> Self {
        PA {
            count: 0,
            sink_cbs: HashMap::new(),
            source_cbs: HashMap::new(),
            device_map: HashMap::new(),
            // on_error_cbs: vec![],
        }
    }
    fn call(&mut self, sink_or_source: SinkOrSource) {
        if let Some(is) = self.device_map.get(&sink_or_source) {
            match sink_or_source {
                SinkOrSource::Sink(s) => {
                    log::debug!("call sink cb");
                    let vinfo = get_sink_vol_by_name(&s).unwrap();
                    is.keys().for_each(|i| {
                        if let Some((_, f)) = self.sink_cbs.get_mut(i) {
                            let mut f = f.borrow_mut();
                            f(&vinfo);
                        }
                    })
                }
                SinkOrSource::Source(s) => {
                    log::debug!("call source cb");
                    let vinfo = get_source_vol_by_name(&s).unwrap();
                    is.keys().for_each(|i| {
                        if let Some((_, f)) = self.source_cbs.get_mut(i) {
                            let mut f = f.borrow_mut();
                            f(&vinfo);
                        }
                    })
                }
            };
        }
    }
    fn add_cb(
        &mut self,
        cb: Box<PaCallback>,
        // error_cb: Option<impl FnMut(String) + 'static>,
        sink_or_source: SinkOrSource,
    ) -> i32 {
        let cb = Rc::new(RefCell::new(cb));
        let key = self.count;
        self.device_map
            .entry(sink_or_source.clone())
            .or_default()
            .insert(key, ());
        match sink_or_source {
            SinkOrSource::Sink(s) => {
                log::debug!("add sink cb");
                if let Some(vi) = get_sink_vol_by_name(&s) {
                    cb.borrow_mut()(&vi);
                }
                self.sink_cbs.insert(key, (s, cb.clone()));
            }
            SinkOrSource::Source(s) => {
                log::debug!("add source cb");
                if let Some(vi) = get_source_vol_by_name(&s) {
                    cb.borrow_mut()(&vi);
                }
                self.source_cbs.insert(key, (s, cb.clone()));
            }
        };
        // if let Some(error_cb) = error_cb {
        //     self.on_error_cbs.push(Box::new(error_cb));
        // };
        self.count += 1;
        key
    }
    // fn add_error_cb(error_cb: impl FnMut(String) + 'static) {
    //     self.on_error_cbs.push(Box::new(error_cb));
    // }
    fn remove_cb(&mut self, key: i32) {
        self.sink_cbs.remove_entry(&key);
        self.source_cbs.remove_entry(&key);
    }
    // fn error(self, e: String) {
    //     log::error!("Pulseaudio error(quit mainloop because of this): {e}");
    //     self.on_error_cbs.into_iter().for_each(|mut f| f(e.clone()));
    // }
}

static IS_PA_INITIALIZED: AtomicBool = AtomicBool::new(false);
static GLOBAL_PA: AtomicPtr<PA> = AtomicPtr::new(std::ptr::null_mut());
fn init_pa() {
    IS_PA_INITIALIZED.store(true, Ordering::Release);
    GLOBAL_PA.store(Box::into_raw(Box::new(PA::new())), Ordering::Release);
}
// fn on_pa_error(e: String) {
//     unsafe {
//         let pa_ptr = PA_CONTEXT.swap(std::ptr::null_mut(), Ordering::Release);
//         if pa_ptr.is_null() {
//             return;
//         }
//         let a = pa_ptr.read();
//         a.error(e);
//     }
// }
fn is_pa_inited() -> bool {
    IS_PA_INITIALIZED.load(Ordering::Acquire)
}
fn is_pa_empty() -> bool {
    GLOBAL_PA.load(Ordering::Acquire).is_null()
}
fn call_pa(sink_or_source: SinkOrSource) {
    unsafe {
        GLOBAL_PA
            .load(Ordering::Acquire)
            .as_mut()
            .unwrap()
            .call(sink_or_source);
    }
}
fn add_cb(
    cb: Box<PaCallback>,
    // error_cb: Option<impl FnMut(String) + 'static>,
    sink_or_source: SinkOrSource,
) -> i32 {
    unsafe {
        GLOBAL_PA
            .load(Ordering::Acquire)
            .as_mut()
            .unwrap()
            // .add_cb(cb, error_cb, sink_or_source)
            .add_cb(cb, sink_or_source)
    }
}
fn rm_cb(key: i32) {
    unsafe {
        GLOBAL_PA
            .load(Ordering::Acquire)
            .as_mut()
            .unwrap()
            .remove_cb(key);
    }
}

pub fn try_init_pulseaudio() -> Result<(), String> {
    if !is_pa_inited() {
        let sr = pa::init_mainloop()?;
        init_pa();
        init_pamixser_thread();
        glib::spawn_future_local(async move {
            log::info!("start pulseaudio signal receiver on glib main thread");
            loop {
                if let Ok(r) = sr.recv().await {
                    log::debug!("recv pulseaudio signal: {r:#?}");
                    match r {
                        Ok(r) => {
                            call_pa(r);
                        }
                        Err(e) => {
                            log::error!("Erro inside pulseaudio mainloop: {e}");
                            break;
                        }
                    }
                } else {
                    log::error!("pulseaudio mainloops seems closed(communication channel closed)");
                    break;
                }
            }
            log::info!("stop pulseaudio signal receiver on glib main thread");
        });
    } else if is_pa_empty() {
        return Err(
            "pulseaudio mainloops seems inited before closed due to some error".to_string(),
        );
    }
    Ok(())
}

#[derive(Debug)]
pub enum OptionalSinkOrSourceDevice {
    Sink(Option<String>),
    Source(Option<String>),
}

#[derive(Debug, Clone)]
pub struct OptionalSinkOrSource(Arc<OptionalSinkOrSourceDevice>);
impl OptionalSinkOrSource {
    pub fn sink(s: Option<String>) -> Self {
        Self(Arc::new(OptionalSinkOrSourceDevice::Sink(s)))
    }
    pub fn source(s: Option<String>) -> Self {
        Self(Arc::new(OptionalSinkOrSourceDevice::Source(s)))
    }
}

pub fn register_callback(
    cb: Box<PaCallback>,
    sos: OptionalSinkOrSource, // cb: impl FnMut(VInfo, InterestMaskSet) + 'static,
                               // error_cb: Option<impl FnMut(String) + 'static>,
                               // sink_or_source: InterestMaskSet,
) -> Result<i32, String> {
    try_init_pulseaudio()?;
    // Ok(add_cb(Box::new(cb), error_cb, sink_or_source))
    let sos = match sos.0.as_ref() {
        OptionalSinkOrSourceDevice::Sink(s) => {
            let s = match s {
                Some(s) => s.clone(),
                None => get_default_sink().ok_or("no default sink")?.to_string(),
            };
            SinkOrSource::Sink(s)
        }
        OptionalSinkOrSourceDevice::Source(s) => {
            let s = match s {
                Some(s) => s.clone(),
                None => get_default_source().ok_or("no default source")?.to_string(),
            };
            SinkOrSource::Source(s)
        }
    };
    let ind = match &sos {
        // SinkOrSource::Sink(s) => (true, match_name_index_sink(s)?),
        // SinkOrSource::Source(s) => (false, match_name_index_source(s)?),
        SinkOrSource::Sink(s) => SinkOrSourceIndex::Sink(match_name_index_sink(s)?),
        SinkOrSource::Source(s) => SinkOrSourceIndex::Source(match_name_index_source(s)?),
    };
    log::debug!("device index: {ind:?}");
    reload_device_vinfo_blocking(ind, None)?;
    Ok(add_cb(cb, sos))
}

pub fn unregister_callback(key: i32) {
    if !is_pa_empty() {
        rm_cb(key);
    }
}

// i don't know how to set it with pulseaudio api
pub fn set_vol(os: OptionalSinkOrSource, v: f64) {
    send_pamixer_signal((os, PaMixerSignalInfo::vol(v)));
}
pub fn set_mute(os: OptionalSinkOrSource, mute: bool) {
    send_pamixer_signal((os, PaMixerSignalInfo::mute(mute)));
}
