use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::Hash,
    sync::{
        atomic::{AtomicBool, AtomicPtr},
        Arc, Mutex,
    },
};

use calloop::channel::Sender;
use system_tray::client::{Client, Event};

use crate::runtime::get_backend_runtime_handle;

use super::{event::TrayEventSignal, item::Tray};

/// destination
pub type TrayMsg = TrayEventSignal;

#[derive(Debug)]
pub struct TrayBackendHandle {
    tray_map: Arc<Mutex<TrayMap>>,
    id: i32,
}
impl TrayBackendHandle {
    pub fn get_tray_map(&self) -> Arc<Mutex<TrayMap>> {
        self.tray_map.clone()
    }
}
impl Drop for TrayBackendHandle {
    fn drop(&mut self) {
        unregister_tray(self.id);
    }
}

#[derive(Debug)]
pub struct TrayMap {
    pub(super) inner: HashMap<Arc<String>, Tray>,
}
impl TrayMap {
    fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            inner: HashMap::new(),
        }))
    }
    pub fn get_tray<Q: Hash + Eq>(&self, destination: &Q) -> Option<&Tray>
    where
        Arc<String>: Borrow<Q>,
    {
        self.inner.get(destination)
    }
}

pub(super) struct TrayContext {
    pub client: Client,
    tray_map: Arc<Mutex<TrayMap>>,
    cbs: HashMap<i32, Sender<TrayMsg>>,
    count: i32,
}
impl TrayContext {
    fn new(client: Client) -> Self {
        Self {
            client,
            cbs: HashMap::new(),
            count: 0,
            tray_map: TrayMap::new(),
        }
    }
    pub fn call(&mut self, e: Event) {
        let mut map = self.tray_map.lock().unwrap();
        if let Some(dest) = map.handle_event(e) {
            drop(map);
            self.cbs.iter().for_each(|(_, cb)| {
                cb.send(dest.clone()).unwrap();
            });
        }
    }
    fn add_cb(&mut self, cb: Sender<TrayMsg>) -> TrayBackendHandle {
        let key = self.count;
        self.count += 1;
        self.cbs.insert(key, cb);

        TrayBackendHandle {
            tray_map: self.tray_map.clone(),
            id: key,
        }
    }
    fn remove_cb(&mut self, key: i32) {
        self.cbs.remove_entry(&key);
    }
}

static TRAY_CONTEXT: AtomicPtr<TrayContext> = AtomicPtr::new(std::ptr::null_mut());
pub(super) fn get_tray_context() -> &'static mut TrayContext {
    unsafe {
        TRAY_CONTEXT
            .load(std::sync::atomic::Ordering::Acquire)
            .as_mut()
            .unwrap()
    }
}
pub fn init_tray_client() {
    static CONTEXT_INITED: AtomicBool = AtomicBool::new(false);

    if CONTEXT_INITED.load(std::sync::atomic::Ordering::Acquire) {
        return;
    }

    let client = get_backend_runtime_handle().block_on(async { Client::new().await.unwrap() });
    let mut tray_rx = client.subscribe();

    TRAY_CONTEXT.store(
        Box::into_raw(Box::new(TrayContext::new(client))),
        std::sync::atomic::Ordering::Release,
    );
    CONTEXT_INITED.store(true, std::sync::atomic::Ordering::Release);

    get_backend_runtime_handle().spawn(async move {
        while let Ok(ev) = tray_rx.recv().await {
            get_tray_context().call(ev);
        }
    });

    // get_main_runtime_handle().spawn();
}

pub fn register_tray(cb: Sender<TrayMsg>) -> TrayBackendHandle {
    get_tray_context().add_cb(cb)
}

pub fn unregister_tray(id: i32) {
    get_tray_context().remove_cb(id);
}
