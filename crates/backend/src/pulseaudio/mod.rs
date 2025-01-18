pub mod change;
mod pa;

use std::{
    collections::{HashMap, HashSet},
    sync::atomic::{AtomicBool, AtomicPtr, Ordering},
};

use calloop::channel::Sender;
pub use pa::PulseAudioDevice;
pub use pa::VInfo;

use crate::runtime::get_backend_runtime_handle;

#[derive(Default)]
struct VInfoMap(HashMap<String, VInfo>);
impl VInfoMap {
    fn get_by_name(&self, n: &str) -> Option<VInfo> {
        let a = self.0.get(n).cloned();
        a
    }
    fn set_by_name(&mut self, n: String, v: VInfo) {
        self.0.insert(n, v);
    }
}

type CallbackID = i32;

struct PA {
    count: i32,
    cbs: HashMap<CallbackID, Sender<VInfo>>,
    device_map: HashMap<PulseAudioDevice, HashSet<CallbackID>>,

    sink_vinfo_map: VInfoMap,
    source_vinfo_map: VInfoMap,
    default_sink: Option<String>,
    default_source: Option<String>,
}

impl PA {
    fn new() -> Self {
        PA {
            count: 0,
            cbs: HashMap::new(),
            device_map: HashMap::new(),

            sink_vinfo_map: VInfoMap::default(),
            source_vinfo_map: VInfoMap::default(),
            default_sink: None,
            default_source: None,
        }
    }
    fn call_device(&mut self, device: &PulseAudioDevice, vinfo: VInfo) {
        self.device_map.get(device).map(|ids| {
            ids.iter().for_each(|id| {
                let cb = self.cbs.get(id).unwrap();
                cb.send(vinfo.clone()).unwrap();
            })
        });
    }
    fn call(&mut self, device: PulseAudioDevice, vinfo: VInfo) {
        match &device {
            PulseAudioDevice::NamedSink(name) => {
                if self.default_sink.as_ref().is_some_and(|dt| dt == name) {
                    self.call_device(&PulseAudioDevice::DefaultSink, vinfo);
                }
                self.sink_vinfo_map.set_by_name(name.clone(), vinfo);
            }
            PulseAudioDevice::NamedSource(name) => {
                if self.default_source.as_ref().is_some_and(|dt| dt == name) {
                    self.call_device(&PulseAudioDevice::DefaultSource, vinfo);
                }
                self.source_vinfo_map.set_by_name(name.clone(), vinfo);
            }
            _ => unreachable!(),
        }
        self.call_device(&device, vinfo);
    }
    fn add_cb(&mut self, cb: Sender<VInfo>, device: PulseAudioDevice) -> i32 {
        let key = self.count;
        self.count += 1;

        match &device {
            PulseAudioDevice::DefaultSink => self
                .default_sink
                .as_ref()
                .and_then(|name| self.sink_vinfo_map.get_by_name(name)),
            PulseAudioDevice::DefaultSource => self
                .default_source
                .as_ref()
                .and_then(|name| self.source_vinfo_map.get_by_name(name)),
            PulseAudioDevice::NamedSink(name) => self.sink_vinfo_map.get_by_name(name),
            PulseAudioDevice::NamedSource(name) => self.source_vinfo_map.get_by_name(name),
        }
        .map(|vinfo| cb.send(vinfo));

        self.cbs.insert(key, cb);
        self.device_map.entry(device).or_default().insert(key);

        key
    }
    fn remove_cb(&mut self, key: i32) {
        self.cbs.remove_entry(&key);
    }
}

static IS_PA_INITIALIZED: AtomicBool = AtomicBool::new(false);
static GLOBAL_PA: AtomicPtr<PA> = AtomicPtr::new(std::ptr::null_mut());
fn init_pa() {
    IS_PA_INITIALIZED.store(true, Ordering::Release);
    GLOBAL_PA.store(Box::into_raw(Box::new(PA::new())), Ordering::Release);
}
fn is_pa_inited() -> bool {
    IS_PA_INITIALIZED.load(Ordering::Acquire)
}
fn get_pa() -> &'static mut PA {
    unsafe { GLOBAL_PA.load(Ordering::Acquire).as_mut().unwrap() }
}

pub fn try_init_pulseaudio() -> Result<(), String> {
    if !is_pa_inited() {
        init_pa();
        pa::init_pulseaudio_subscriber();
    }
    Ok(())
}

pub fn register_callback(cb: Sender<VInfo>, device: PulseAudioDevice) -> Result<i32, String> {
    get_backend_runtime_handle().block_on(async move {
        try_init_pulseaudio()?;
        Ok(get_pa().add_cb(cb, device))
    })
}

pub fn unregister_callback(key: i32) {
    get_backend_runtime_handle().block_on(async move {
        if is_pa_inited() {
            get_pa().remove_cb(key);
        }
    })
}
