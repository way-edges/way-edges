pub mod change;
mod pa;

use std::{
    collections::{HashMap, HashSet},
    sync::atomic::{AtomicBool, AtomicPtr, Ordering},
};

use gtk::glib;
pub use pa::PulseAudioDevice;
use pa::VInfo;

pub type PaCallback = dyn FnMut(&VInfo);

type CallbackID = i32;

struct PA {
    count: i32,
    cbs: HashMap<CallbackID, Box<PaCallback>>,
    device_map: HashMap<PulseAudioDevice, HashSet<CallbackID>>,
}

impl PA {
    fn new() -> Self {
        PA {
            count: 0,
            cbs: HashMap::new(),
            device_map: HashMap::new(),
        }
    }
    fn call(&mut self, device: PulseAudioDevice) {
        if let Some(ids) = self.device_map.get(&device) {
            ids.iter().for_each(|id| {
                if let (Some(cb), Some(vinfo)) = (self.cbs.get_mut(id), device.get_vinfo()) {
                    cb(&vinfo);
                }
            });
        }
    }
    fn add_cb(&mut self, mut cb: Box<PaCallback>, device: PulseAudioDevice) -> i32 {
        let key = self.count;
        self.count += 1;

        if let Some(vinfo) = device.get_vinfo() {
            cb(&vinfo)
        }

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
fn is_pa_empty() -> bool {
    GLOBAL_PA.load(Ordering::Acquire).is_null()
}
fn call_pa(device: PulseAudioDevice) {
    unsafe {
        GLOBAL_PA
            .load(Ordering::Acquire)
            .as_mut()
            .unwrap()
            .call(device);
    }
}
fn add_cb(cb: Box<PaCallback>, device: PulseAudioDevice) -> i32 {
    unsafe {
        GLOBAL_PA
            .load(Ordering::Acquire)
            .as_mut()
            .unwrap()
            .add_cb(cb, device)
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
        log::info!("start init pulseaudio related stuff");
        pa::init_pulseaudio_subscriber();
        init_pa();
        glib::spawn_future_local(async move {
            log::info!("start pulseaudio signal receiver on glib main thread");
            let sr = &pa::PaSignalChannel.1;
            loop {
                if let Ok(r) = sr.recv().await {
                    log::debug!("recv pulseaudio signal: {r:#?}");
                    call_pa(r);
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

pub fn register_callback(cb: Box<PaCallback>, device: PulseAudioDevice) -> Result<i32, String> {
    try_init_pulseaudio()?;
    log::info!("register pulseaudio callback for device: {device:?}");
    Ok(add_cb(cb, device))
}

pub fn unregister_callback(key: i32) {
    log::info!("unregister pulseaudio callback for key: {key:?}");
    if !is_pa_empty() {
        rm_cb(key);
    }
}
