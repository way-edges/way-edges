use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicPtr, Ordering},
        Arc, OnceLock, RwLock,
    },
};

use gtk::glib;
use libpulse_binding::{
    self as pulse,
    callbacks::ListResult,
    context::{
        introspect::{ServerInfo, SinkInfo, SourceInfo},
        subscribe::{Facility, InterestMaskSet, Operation},
        Context, FlagSet,
    },
    volume::{ChannelVolumes, Volume},
};

fn get_avg_volume(cv: ChannelVolumes) -> f64 {
    cv.avg().0 as f64 / Volume::NORMAL.0 as f64
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum PulseAudioDevice {
    DefaultSink,
    DefaultSource,
    NamedSink(String),
    NamedSource(String),
}
impl PulseAudioDevice {
    pub fn get_vinfo(&self) -> Option<VInfo> {
        match self {
            PulseAudioDevice::DefaultSink => {
                get_default_sink().and_then(|name| get_sink_vol_by_name(name))
            }
            PulseAudioDevice::DefaultSource => {
                get_default_source().and_then(|name| get_source_vol_by_name(name))
            }
            PulseAudioDevice::NamedSink(name) => get_sink_vol_by_name(name),
            PulseAudioDevice::NamedSource(name) => get_source_vol_by_name(name),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
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
    a
}
pub fn get_source_vol_by_name(n: &str) -> Option<VInfo> {
    log::debug!("get_source_vol_by_name {}", n);
    let a = get_vinfos().read().unwrap().1.get(n).cloned();
    a
}
pub fn set_sink_vol_by_name(n: String, v: VInfo) {
    log::debug!("set_sink_vol_by_name {}", n);
    get_vinfos().write().unwrap().0.insert(n, v);
}
pub fn set_source_vol_by_name(n: String, v: VInfo) {
    log::debug!("set_source_vol_by_name {}", n);
    get_vinfos().write().unwrap().1.insert(n, v);
}

static MAINLOOP: AtomicPtr<libpulse_glib_binding::Mainloop> = AtomicPtr::new(std::ptr::null_mut());
static CONTEXT: AtomicPtr<Context> = AtomicPtr::new(std::ptr::null_mut());

fn init_mainloop_and_context() -> &'static mut Context {
    let m = libpulse_glib_binding::Mainloop::new(None).unwrap();
    let c = Context::new(&m, "Volume Monitor").expect("Failed to create context");
    MAINLOOP.store(Box::into_raw(Box::new(m)), Ordering::Release);
    CONTEXT.store(Box::into_raw(Box::new(c)), Ordering::Release);
    unsafe { CONTEXT.load(Ordering::Acquire).as_mut().unwrap() }
}
pub fn with_context<T>(f: impl FnOnce(&mut Context) -> T) -> T {
    let a = unsafe { CONTEXT.load(Ordering::Acquire).as_mut().unwrap() };
    f(a)
}

pub fn process_list_result<'a, T: 'a>(ls: ListResult<&'a T>) -> Option<&'a T> {
    match ls {
        pulse::callbacks::ListResult::Item(res) => {
            return Some(res);
        }
        pulse::callbacks::ListResult::End => {}
        pulse::callbacks::ListResult::Error => {
            log::error!("Error getting list result info");
        }
    };
    None
}

use util::notify_send;

use super::get_pa;

fn signal_callback_group(msg: PulseAudioDevice) {
    // NOTE: WE USE LIBPULSE WITH GLIB BINDING
    // SO NO NEED TO WORRY ABOUT SEND OR SYNC
    get_pa().call(msg);
}

pub fn sink_cb(list_result: ListResult<&SinkInfo>) {
    let device_name = if let Some(sink_info) = process_list_result(list_result) {
        let avg = get_avg_volume(sink_info.volume);
        let desc = sink_info.name.clone().unwrap().to_string();
        set_sink_vol_by_name(
            desc.clone(),
            VInfo {
                vol: avg,
                is_muted: sink_info.mute,
            },
        );
        desc
    } else {
        return;
    };

    if let Some(default) = get_default_sink() {
        if &device_name == default {
            signal_callback_group(PulseAudioDevice::DefaultSink);
        }
    }
    signal_callback_group(PulseAudioDevice::NamedSink(device_name))
}

pub fn source_cb(list_result: ListResult<&SourceInfo>) {
    let device_name = if let Some(source_info) = process_list_result(list_result) {
        let avg = get_avg_volume(source_info.volume);
        let desc = source_info.name.clone().unwrap().to_string();
        set_source_vol_by_name(
            desc.clone(),
            VInfo {
                vol: avg,
                is_muted: source_info.mute,
            },
        );
        desc
    } else {
        return;
    };

    if let Some(default) = get_default_source() {
        if &device_name == default {
            signal_callback_group(PulseAudioDevice::DefaultSource)
        }
    }
    signal_callback_group(PulseAudioDevice::NamedSource(device_name))
}

fn server_cb(server_info: &ServerInfo) {
    if let Some(name) = &server_info.default_sink_name {
        let run = || {
            set_default_sink(name.to_string());
            with_context(|ctx| {
                ctx.introspect().get_sink_info_by_name(name, sink_cb);
            });
        };

        match get_default_sink() {
            Some(default) if default != name => run(),
            None => run(),
            _ => {}
        };
    }

    if let Some(name) = &server_info.default_source_name {
        let run = || {
            set_default_source(name.to_string());
            with_context(|ctx| {
                ctx.introspect().get_source_info_by_name(name, source_cb);
            });
        };

        match get_default_source() {
            Some(default) if default != name => run(),
            None => run(),
            _ => {}
        };
    };
}

pub fn subscribe_cb(facility: Option<Facility>, _: Option<Operation>, index: u32) {
    let facility = if let Some(facility) = facility {
        facility
    } else {
        return;
    };

    with_context(|ctx| {
        let ins = ctx.introspect();
        match facility {
            Facility::Sink => {
                ins.get_sink_info_by_index(index, sink_cb);
            }
            Facility::Source => {
                ins.get_source_info_by_index(index, source_cb);
            }
            Facility::Server => {
                ins.get_server_info(server_cb);
            }
            _ => {}
        };
    });
}

fn context_connect(ctx: &mut Context) {
    ctx.connect(None, FlagSet::NOAUTOSPAWN, None)
        .map_err(|e| format!("Failed to connect context: {e}"))
        .unwrap();

    ctx.set_state_callback(Some(Box::new(move || {
        with_context(|ctx| match ctx.get_state() {
            pulse::context::State::Failed => {
                log::error!("Fail to connect pulseaudio context, retry in ");
                glib::timeout_add_seconds_once(3, init_pulseaudio_subscriber);
            }
            pulse::context::State::Terminated => {
                let msg = "Pulseaudio terminated, wtf happened?!";
                log::error!("msg");
                notify_send("Pulseaudio terminated", msg, true);
            }
            pulse::context::State::Ready => with_pulse_audio_connected(ctx),
            _ => {
                log::warn!("Unknow state");
            }
        });
    })));
}

fn setup_subscribe(ctx: &mut Context) {
    ctx.subscribe(
        InterestMaskSet::SINK | InterestMaskSet::SOURCE | InterestMaskSet::SERVER,
        move |s| {
            if !s {
                log::warn!("Fail to subscribe pulseaudio");
            }
        },
    );

    ctx.set_subscribe_callback(Some(Box::new(subscribe_cb)));
}

fn get_initial_data(ctx: &mut Context) {
    let ins = ctx.introspect();
    ins.get_server_info(server_cb);
    ins.get_sink_info_list(sink_cb);
    ins.get_source_info_list(source_cb);
}

fn with_pulse_audio_connected(ctx: &mut Context) {
    log::debug!("start subscribe pulseaudio sink and source");
    setup_subscribe(ctx);
    get_initial_data(ctx);
}

pub fn init_pulseaudio_subscriber() {
    let ctx = init_mainloop_and_context();
    context_connect(ctx);
}
