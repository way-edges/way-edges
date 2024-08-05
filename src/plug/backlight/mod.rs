mod dbus;
mod watch;

use std::{
    borrow::Cow,
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::atomic::{AtomicBool, AtomicPtr, Ordering},
};

use blight::Device;
use dbus::set_brightness;
use watch::WatchCtx;

pub type PaCallback = dyn FnMut(f64);

struct BackLight {
    count: i32,
    cbs: HashMap<i32, (String, WatchCtx, Rc<RefCell<PaCallback>>)>,
    device_map: HashMap<String, (Device, HashMap<i32, ()>)>,

    v: HashMap<String, f64>,
}

impl BackLight {
    fn new() -> Self {
        Self {
            count: 0,
            cbs: HashMap::new(),
            device_map: HashMap::new(),
            v: HashMap::new(),
        }
    }
    fn call(&mut self, device: &str) {
        log::debug!("call backlight cb");
        if let Some((d, v)) = self.device_map.get_mut(device) {
            d.reload();
            v.keys().for_each(|i| {
                if let Some((_, _, f)) = self.cbs.get(i) {
                    f.borrow_mut()(d.current_percent());
                }
            });
        }
    }
    fn add_cb(
        &mut self,
        cb: Box<PaCallback>,
        device: Device,
        ctx: WatchCtx,
    ) -> Result<i32, String> {
        let cb = Rc::new(RefCell::new(cb));
        let key = self.count;
        log::debug!("add backlight cb");

        let name = device.name().to_string();
        self.cbs.insert(key, (name.clone(), ctx, cb.clone()));
        if let Some((_, v)) = self.device_map.get_mut(&name) {
            v.insert(key, ());
        } else {
            let de = Device::new(Some(Cow::from(&name)))
                .map_err(|f| format!("Failed to create device: {f}"))?;
            self.device_map
                .insert(name.clone(), (de, HashMap::from([(key, ())])));
        }

        cb.borrow_mut()(*self.v.get(&name).unwrap());
        self.count += 1;
        Ok(key)
    }
    fn remove_cb(&mut self, key: i32) {
        if let Some((d, _, _)) = self.cbs.remove(&key) {
            if let Some((_, h)) = self.device_map.get_mut(&d) {
                h.remove(&key);
            };
        }
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
fn init_bl() {
    IS_BL_INITED.store(true, Ordering::Release);
    BL_CTX.store(Box::into_raw(Box::new(BackLight::new())), Ordering::Release);
}
fn is_bl_inited() -> bool {
    IS_BL_INITED.load(Ordering::Acquire)
}
fn add_cb(cb: Box<PaCallback>, device: Device, ctx: WatchCtx) -> Result<i32, String> {
    unsafe { get_ctx().as_mut().unwrap().add_cb(cb, device, ctx) }
}
fn rm_cb(key: i32) {
    unsafe {
        get_ctx().as_mut().unwrap().remove_cb(key);
    }
}
fn call_cb(name: &str) {
    unsafe { get_ctx().as_mut().unwrap().call(name) }
}

pub fn try_init_backlight(device: Device) -> Result<WatchCtx, String> {
    if !is_bl_inited() {
        init_bl();
    }
    init_watcher(device)
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

fn init_watcher(device: Device) -> Result<WatchCtx, String> {
    let path_buf = device.device_path();
    let ctx = watch::watch(&path_buf)?;
    update_backlight(&device);
    let recv = ctx.r.clone();
    use gtk::glib;
    glib::spawn_future_local(async move {
        loop {
            let res = recv.recv().await;
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
        }
    });
    Ok(ctx)
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
    let device = match_device(device_name)?;

    let ctx = try_init_backlight(device.clone())?;

    add_cb(Box::new(cb), device, ctx)
}

pub fn unregister_callback(key: i32) {
    if is_bl_inited() {
        rm_cb(key);
    }
}
