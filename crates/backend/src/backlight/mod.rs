pub mod dbus;
mod watch;

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicBool, AtomicPtr, Ordering},
        Arc,
    },
};

use blight::Device;

pub type PaCallback = Box<dyn FnMut(f64)>;

struct BackLight {
    count: i32,
    cbs: HashMap<i32, (Arc<String>, PaCallback)>,
    device_map: HashMap<Arc<String>, (Device, HashSet<i32>)>,
}

impl BackLight {
    fn new() -> Self {
        Self {
            count: 0,
            cbs: HashMap::new(),
            device_map: HashMap::new(),
        }
    }
    fn call(&mut self, device_name: Arc<String>) {
        if let Some((device, callback_ids)) = self.device_map.get_mut(&device_name) {
            device.reload();

            let v = device.current_percent() / 100.;
            callback_ids.iter().for_each(|i| {
                if let Some((_, f)) = self.cbs.get_mut(i) {
                    f(v);
                }
            });
        }
    }
    fn add_cb(&mut self, mut cb: PaCallback, mut device: Device) -> Result<i32, String> {
        device.reload();

        let id = self.count;
        let name = Arc::new(device.name().to_string());

        cb(device.current_percent() / 100.);

        self.cbs.insert(id, (name.clone(), cb));
        if let Some((_, id_set)) = self.device_map.get_mut(&name) {
            id_set.insert(self.count);
        } else {
            watch::watch(device.device_path().as_path(), name.clone());
            self.device_map.insert(name, (device, HashSet::from([id])));
        }

        self.count += 1;
        Ok(id)
    }
    fn remove_cb(&mut self, key: i32) {
        if let Some((name, _)) = self.cbs.remove(&key) {
            let (device, id_set) = self.device_map.get_mut(&name).unwrap();
            id_set.remove(&key);
            if id_set.is_empty() {
                // drop(id_set);
                watch::unwatch(device.device_path().as_path());
                self.device_map.remove(&name);
            }
        }
    }
}

static IS_BL_INITED: AtomicBool = AtomicBool::new(false);
static BL_CTX: AtomicPtr<BackLight> = AtomicPtr::new(std::ptr::null_mut());
fn get_ctx() -> &'static mut BackLight {
    unsafe { BL_CTX.load(Ordering::Acquire).as_mut().unwrap() }
}
fn is_bl_inited() -> bool {
    IS_BL_INITED.load(Ordering::Acquire)
}

pub fn try_init_backlight() {
    if !is_bl_inited() {
        IS_BL_INITED.store(true, Ordering::Release);
        BL_CTX.store(Box::into_raw(Box::new(BackLight::new())), Ordering::Release);
        init_watcher()
    }
}

fn init_watcher() {
    let r = watch::init_watcher();
    gtk::glib::spawn_future_local(async move {
        while let Ok(name) = r.recv().await {
            get_ctx().call(name);
        }
    });
}

fn match_device(device_name: Option<String>) -> Result<Device, String> {
    match device_name.clone() {
        Some(s) => Device::new(Some(Cow::from(s))),
        None => Device::new(None),
    }
    .map_err(|e| format!("Failed to get device({device_name:?}): {e}"))
}

pub fn register_callback(
    cb: impl FnMut(f64) + 'static,
    device_name: Option<String>,
) -> Result<i32, String> {
    try_init_backlight();

    let device = match_device(device_name)?;
    get_ctx().add_cb(Box::new(cb), device)
}

pub fn unregister_callback(key: i32) {
    log::info!("unregister backlight callback for key({key})");
    if is_bl_inited() {
        get_ctx().remove_cb(key);
    }
}
