mod dbus;
mod watch;

use std::{
    borrow::Cow,
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::atomic::{AtomicBool, AtomicPtr, Ordering},
    thread::{self},
};

use blight::Device;
use dbus::set_brightness;

pub type PaCallback = dyn FnMut(f64);
pub type PaErrCallback = dyn FnMut(String);

struct BackLight {
    count: i32,
    cbs: HashMap<i32, (String, Rc<RefCell<PaCallback>>)>,
    device_map: HashMap<String, (Device, HashMap<i32, ()>)>,
    on_error_cbs: Vec<Box<PaErrCallback>>,

    v: HashMap<String, f64>,
}

impl BackLight {
    fn new() -> Self {
        Self {
            count: 0,
            cbs: HashMap::new(),
            device_map: HashMap::new(),
            on_error_cbs: vec![],
            v: HashMap::new(),
        }
    }
    fn call(&mut self, device: &str) {
        log::debug!("call backlight cb");
        if let Some((d, v)) = self.device_map.get_mut(device) {
            d.reload();
            v.keys().for_each(|i| {
                if let Some((_, f)) = self.cbs.get(i) {
                    f.borrow_mut()(d.current_percent());
                }
            });
        }
    }
    fn add_cb(
        &mut self,
        cb: Box<PaCallback>,
        error_cb: Option<impl FnMut(String) + 'static>,
        device: Device,
    ) -> Result<i32, String> {
        let cb = Rc::new(RefCell::new(cb));
        let key = self.count;
        log::debug!("add sink cb");

        let name = device.name().to_string();
        self.cbs.insert(key, (name.clone(), cb.clone()));
        if let Some((_, v)) = self.device_map.get_mut(&name) {
            v.insert(key, ());
        } else {
            let de = Device::new(Some(Cow::from(&name)))
                .map_err(|f| format!("Failed to create device: {f}"))?;
            self.device_map
                .insert(name.clone(), (de, HashMap::from([(key, ())])));
        }

        cb.borrow_mut()(*self.v.get(&name).unwrap());
        if let Some(error_cb) = error_cb {
            self.on_error_cbs.push(Box::new(error_cb));
        };
        self.count += 1;
        Ok(key)
    }
    fn remove_cb(&mut self, key: i32) {
        if let Some((d, _)) = self.cbs.remove(&key) {
            if let Some((_, h)) = self.device_map.get_mut(&d) {
                h.remove(&key);
                // if h.is_empty() {
                //     self.device_map.remove(&d);
                // };
            };
        }
    }
    fn error(self, e: String) {
        log::error!("Pulseaudio error(quit mainloop because of this): {e}");
        self.on_error_cbs.into_iter().for_each(|mut f| f(e.clone()));
    }
    fn update_v(&mut self, device_name: String, v: f64) {
        self.v.insert(device_name, v);
    }
    fn set_v(&self, device_name: String, f: f64) {
        use gtk::glib;
        if let Some((device, _)) = self.device_map.get(&device_name) {
            let p = (device.max() as f64) * f;
            glib::spawn_future_local(async move {
                if let Err(e) = set_brightness(&device_name, p as u32).await {
                    log::error!("Error setting brightness: {e}");
                } else {
                    call_cb(&device_name);
                };
            });
        }
    }
}

static IS_BL_INITED: AtomicBool = AtomicBool::new(false);
static BL_CTX: AtomicPtr<BackLight> = AtomicPtr::new(std::ptr::null_mut());
unsafe fn get_ctx() -> *mut BackLight {
    BL_CTX.load(Ordering::Acquire)
}
fn init_pa() {
    IS_BL_INITED.store(true, Ordering::Release);
    BL_CTX.store(Box::into_raw(Box::new(BackLight::new())), Ordering::Release);
}
fn on_pa_error(e: String) {
    unsafe {
        let pa_ptr = BL_CTX.swap(std::ptr::null_mut(), Ordering::Release);
        if pa_ptr.is_null() {
            return;
        }
        let a = pa_ptr.read();
        a.error(e);
    }
}
fn is_pa_inited() -> bool {
    IS_BL_INITED.load(Ordering::Acquire)
}
fn is_pa_empty() -> bool {
    BL_CTX.load(Ordering::Acquire).is_null()
}
fn add_cb(
    cb: Box<PaCallback>,
    error_cb: Option<impl FnMut(String) + 'static>,
    device: Device,
) -> Result<i32, String> {
    unsafe { get_ctx().as_mut().unwrap().add_cb(cb, error_cb, device) }
}
fn rm_cb(key: i32) {
    unsafe {
        get_ctx().as_mut().unwrap().remove_cb(key);
    }
}
fn call_cb(name: &str) {
    unsafe { get_ctx().as_mut().unwrap().call(name) }
}

pub fn try_init_backlight(device: Device) -> Result<(), String> {
    if !is_pa_inited() {
        init_pa();
    } else if is_pa_empty() {
        return Err(
            "pulseaudio mainloops seems inited before closed due to some error".to_string(),
        );
    }
    init_watcher(device)?;
    Ok(())
}

fn update_backlight(device: &Device) {
    unsafe {
        get_ctx()
            .as_mut()
            .unwrap()
            .update_v(device.name().to_string(), device.current_percent())
    }
}

pub fn set_backlight(device_name: Option<String>, v: f64) -> Result<(), String> {
    let device_name = match_device(device_name)?.name().to_string();
    unsafe {
        get_ctx().as_mut().unwrap().set_v(device_name, v);
    }
    Ok(())
}

fn init_watcher(device: Device) -> Result<(), String> {
    let path_buf = device.device_path();
    let r = watch::watch(&path_buf)?;
    update_backlight(&device);
    thread::spawn(move || loop {
        let res = r.recv_blocking();
        match res {
            Ok(s) => match s {
                Ok(_) => update_backlight(&device),
                Err(e) => {
                    log::error!("Watch error on device({}): {e}", device.name());
                }
            },
            Err(e) => {
                log::warn!("watcher for device({}) is closed: {e}", device.name());
                break;
            }
        }
    });
    Ok(())
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
    error_cb: Option<impl FnMut(String) + 'static>,
    device_name: Option<String>,
) -> Result<i32, String> {
    let device = match_device(device_name)?;

    try_init_backlight(device.clone())?;

    add_cb(Box::new(cb), error_cb, device)
}

pub fn unregister_callback(key: i32) {
    if !is_pa_empty() {
        rm_cb(key);
    }
}
