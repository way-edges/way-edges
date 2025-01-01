use std::{
    collections::HashMap,
    path::Path,
    sync::{
        atomic::{AtomicPtr, Ordering},
        Arc,
    },
};

use async_channel::Receiver;
use notify::{INotifyWatcher, Watcher};

struct WatcherCtx {
    watcher: INotifyWatcher,
    path_to_device_name: HashMap<String, Arc<String>>,
}
impl WatcherCtx {
    fn match_path_to_device_name(&self, path: &str) -> Option<Arc<String>> {
        self.path_to_device_name.get(path).cloned()
    }
    fn watch(&mut self, path: &Path, name: Arc<String>) {
        if self
            .path_to_device_name
            .insert(path.to_string_lossy().to_string(), name)
            .is_none()
        {
            self.watcher
                .watch(path, notify::RecursiveMode::NonRecursive)
                .unwrap();
        }
    }
    fn unwatch(&mut self, path: &Path) {
        if self
            .path_to_device_name
            .remove(path.to_string_lossy().as_ref())
            .is_some()
        {
            self.watcher.unwatch(path).unwrap();
        }
    }
}

// NOTE: Another notify-rs monitor backlight file, cost 2 threads
static WATCHER: AtomicPtr<WatcherCtx> = AtomicPtr::new(std::ptr::null_mut());
fn get_watcher() -> &'static mut WatcherCtx {
    unsafe { WATCHER.load(Ordering::Acquire).as_mut().unwrap() }
}

pub(super) fn init_watcher() -> Receiver<Arc<String>> {
    let (s, r) = async_channel::bounded(1);

    let watcher = notify::recommended_watcher(move |e: notify::Result<notify::Event>| {
        match e {
            Ok(event) => match event.kind {
                notify::EventKind::Create(_) | notify::EventKind::Modify(_) => {
                    if let Some(device_name) = event
                        .paths
                        .first()
                        .and_then(|f| f.parent())
                        .and_then(|device_path| {
                            get_watcher().match_path_to_device_name(device_path.to_str().unwrap())
                        })
                    {
                        s.force_send(device_name).unwrap();
                    }
                }
                _ => {}
            },
            Err(e) => {
                log::error!("error watch backlight file: {e}");
            }
        };
    })
    .unwrap();

    WATCHER.store(
        Box::into_raw(Box::new(WatcherCtx {
            watcher,
            path_to_device_name: HashMap::new(),
        })),
        Ordering::Release,
    );

    r
}

pub(super) fn watch(p: &Path, name: Arc<String>) {
    get_watcher().watch(p, name.clone());
}

pub(super) fn unwatch(p: &Path) {
    get_watcher().unwatch(p);
}
