use std::sync::atomic::{AtomicPtr, Ordering};

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

fn set_default_sink(s: String) {
    get_pa().default_sink.replace(s);
}
fn set_default_source(s: String) {
    get_pa().default_source.replace(s);
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum PulseAudioDevice {
    DefaultSink,
    DefaultSource,
    NamedSink(String),
    NamedSource(String),
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct VInfo {
    pub vol: f64,
    pub is_muted: bool,
}

static CONTEXT: AtomicPtr<Context> = AtomicPtr::new(std::ptr::null_mut());

pub fn with_context<T>(f: impl FnOnce(&mut Context) -> T) -> T {
    let a = unsafe { CONTEXT.load(Ordering::Acquire).as_mut().unwrap() };
    f(a)
}

pub fn drain_list<'a, T: 'a>(ls: ListResult<&'a T>) -> Option<&'a T> {
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

use libpulse_tokio::TokioMain;

use crate::runtime::get_backend_runtime;

use super::get_pa;

fn signal_callback_group(msg: PulseAudioDevice, vinfo: VInfo) {
    // NOTE: WE USE LIBPULSE WITH GLIB BINDING
    // SO NO NEED TO WORRY ABOUT SEND OR SYNC
    get_pa().call(msg, vinfo);
}

pub fn sink_cb(list_result: ListResult<&SinkInfo>) {
    if let Some(sink_info) = drain_list(list_result) {
        let avg = get_avg_volume(sink_info.volume);
        let desc = sink_info.name.clone().unwrap().to_string();
        signal_callback_group(
            PulseAudioDevice::NamedSink(desc),
            VInfo {
                vol: avg,
                is_muted: sink_info.mute,
            },
        )
    };
}

pub fn source_cb(list_result: ListResult<&SourceInfo>) {
    if let Some(source_info) = drain_list(list_result) {
        let avg = get_avg_volume(source_info.volume);
        let desc = source_info.name.clone().unwrap().to_string();

        signal_callback_group(
            PulseAudioDevice::NamedSource(desc),
            VInfo {
                vol: avg,
                is_muted: source_info.mute,
            },
        );
    };
}

fn server_cb(server_info: &ServerInfo) {
    if let Some(name) = &server_info.default_sink_name {
        set_default_sink(name.to_string());
        with_context(|ctx| {
            ctx.introspect().get_sink_info_by_name(name, sink_cb);
        });
    }

    if let Some(name) = &server_info.default_source_name {
        set_default_source(name.to_string());
        with_context(|ctx| {
            ctx.introspect().get_source_info_by_name(name, source_cb);
        });
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

fn init_mainloop_and_context() -> (TokioMain, &'static mut Context) {
    let m = libpulse_tokio::TokioMain::new();
    let c = Context::new(&m, "Volume Monitor").expect("Failed to create context");
    CONTEXT.store(Box::into_raw(Box::new(c)), Ordering::Release);

    (m, unsafe {
        CONTEXT.load(Ordering::Acquire).as_mut().unwrap()
    })
}

pub fn init_pulseaudio_subscriber() {
    get_backend_runtime().spawn_local(async {
        let (mut m, ctx) = init_mainloop_and_context();

        ctx.connect(None, FlagSet::NOFAIL, None)
            .map_err(|e| format!("Failed to connect context: {e}"))
            .unwrap();

        match m.wait_for_ready(ctx).await {
            Ok(pulse::context::State::Ready) => {}
            Ok(c) => {
                log::error!("Pulse context {:?}, not continuing", c);
            }
            Err(_) => {
                log::error!("Pulse mainloop exited while waiting on context, not continuing");
            }
        }

        with_pulse_audio_connected(ctx);

        let res = m.run().await;

        log::error!("Pulse mainloop exited with retval: {res:?}");
    });
}
