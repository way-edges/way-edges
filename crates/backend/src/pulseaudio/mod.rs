pub mod change;
mod pa;

use std::{
    collections::{HashMap, HashSet},
    sync::atomic::{AtomicBool, AtomicPtr, Ordering},
};

pub use pa::PulseAudioDevice;
pub use pa::VInfo;

pub type PaCallback = Box<dyn FnMut(&VInfo)>;

type CallbackID = i32;

struct PA {
    count: i32,
    cbs: HashMap<CallbackID, PaCallback>,
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
    fn add_cb(&mut self, mut cb: PaCallback, device: PulseAudioDevice) -> i32 {
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

pub fn register_callback(
    cb: impl FnMut(&VInfo) + 'static,
    device: PulseAudioDevice,
) -> Result<i32, String> {
    try_init_pulseaudio()?;
    Ok(get_pa().add_cb(Box::new(cb), device))
}

pub fn unregister_callback(key: i32) {
    if is_pa_inited() {
        get_pa().remove_cb(key);
    }
}
