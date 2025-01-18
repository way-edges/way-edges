pub mod dbus;

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::atomic::{AtomicBool, AtomicPtr, Ordering},
};

use blight::Device;
use calloop::channel::Sender;
use futures_util::StreamExt;
use inotify::{Inotify, WatchDescriptor, WatchMask, Watches};

use crate::runtime::get_backend_runtime_handle;

type ID = i32;
type DeviceName = Rc<String>;

struct BackLight {
    count: i32,
    cbs: HashMap<ID, (DeviceName, Sender<f64>)>,
    device_map: HashMap<DeviceName, (WatchDescriptor, Device, HashSet<ID>)>,

    watcher: Watches,
    path_to_device_name: HashMap<WatchDescriptor, DeviceName>,
}

impl BackLight {
    fn new(watcher: Watches) -> Self {
        Self {
            count: 0,
            cbs: HashMap::new(),
            device_map: HashMap::new(),
            watcher,
            path_to_device_name: HashMap::new(),
        }
    }
    fn call(&mut self, fd: WatchDescriptor) {
        #[allow(clippy::option_map_unit_fn)]
        self.path_to_device_name
            .get(&fd)
            .and_then(|name| self.device_map.get_mut(name))
            .map(|(_, device, ids)| {
                device.reload();
                let v = device.current_percent() / 100.;
                ids.iter().for_each(|i| {
                    if let Some((_, s)) = self.cbs.get(i) {
                        s.send(v).unwrap();
                    }
                });
            });
    }
    fn add_cb(&mut self, cb: Sender<f64>, mut device: Device) -> Result<i32, String> {
        device.reload();

        let id = self.count;
        let name = Rc::new(device.name().to_string());

        cb.send(device.current_percent() / 100.).unwrap();

        self.cbs.insert(id, (name.clone(), cb));
        if let Some((_, _, id_set)) = self.device_map.get_mut(&name) {
            id_set.insert(self.count);
        } else {
            let fd = self
                .watcher
                .add(
                    device.device_path().as_path(),
                    WatchMask::CREATE | WatchMask::MODIFY | WatchMask::DELETE,
                )
                .unwrap();
            self.device_map
                .insert(name.clone(), (fd.clone(), device, HashSet::from([id])));
            self.path_to_device_name.insert(fd, name);
        }

        self.count += 1;
        Ok(id)
    }
    fn remove_cb(&mut self, key: i32) {
        if let Some((name, _)) = self.cbs.remove(&key) {
            let (_, _, id_set) = self.device_map.get_mut(&name).unwrap();
            id_set.remove(&key);
            if id_set.is_empty() {
                let (fd, _, _) = self.device_map.remove(&name).unwrap();
                self.watcher.remove(fd).unwrap();
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
        let watcher = init_watcher();
        BL_CTX.store(
            Box::into_raw(Box::new(BackLight::new(watcher))),
            Ordering::Release,
        );
    }
}

fn init_watcher() -> Watches {
    let inotify = Inotify::init().unwrap();
    let watches = inotify.watches();

    get_backend_runtime_handle().spawn(async move {
        let mut buffer = [0; 1024];
        let mut stream = inotify.into_event_stream(&mut buffer).unwrap();
        while let Some(event_or_error) = stream.next().await {
            let event = event_or_error.unwrap();
            get_ctx().call(event.wd);
        }
    });

    watches
}

fn match_device(device_name: Option<&String>) -> Result<Device, String> {
    match device_name {
        Some(s) => Device::new(Some(Cow::from(s))),
        None => Device::new(None),
    }
    .map_err(|e| format!("Failed to get device({device_name:?}): {e}"))
}

pub fn register_callback(cb: Sender<f64>, device_name: Option<String>) -> Result<i32, String> {
    get_backend_runtime_handle().block_on(async {
        try_init_backlight();

        let device = match_device(device_name.as_ref())?;
        get_ctx().add_cb(cb, device)
    })
}

pub fn unregister_callback(key: i32) {
    get_backend_runtime_handle().block_on(async {
        log::info!("unregister backlight callback for key({key})");
        if is_bl_inited() {
            get_ctx().remove_cb(key);
        }
    })
}
