use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicBool, AtomicPtr},
        Arc, Mutex,
    },
};

use calloop::channel::Sender;
use system_tray::client::{Client, Event};

use crate::{
    runtime::get_backend_runtime_handle,
    tray::item::{Icon, RootMenu},
};

use super::{event::TrayEventSignal, item::Tray};

/// destination
pub type TrayMsg = TrayEventSignal;

#[derive(Debug)]
pub struct TrayBackendHandle {
    tray_map: TrayMap,
    id: i32,
}
impl TrayBackendHandle {
    pub fn get_tray_map(&mut self) -> &mut TrayMap {
        &mut self.tray_map
    }
}
impl Drop for TrayBackendHandle {
    fn drop(&mut self) {
        unregister_tray(self.id);
    }
}

// frontend and backend uses different TrayMap
// but shares the same key and value
#[derive(Debug, Clone, Default)]
pub struct TrayMap(HashMap<Arc<String>, Arc<Mutex<Tray>>>);
impl Deref for TrayMap {
    type Target = HashMap<Arc<String>, Arc<Mutex<Tray>>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for TrayMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TrayMap {
    pub fn add_tray(&mut self, dest: Arc<String>, tray: Arc<Mutex<Tray>>) {
        self.insert(dest, tray);
    }

    pub fn remove_tray(&mut self, destination: &Arc<String>) {
        self.remove(destination);
    }

    // this only called in the backend runtime thread
    pub(super) fn handle_event(
        &mut self,
        e: system_tray::client::Event,
    ) -> Option<TrayEventSignal> {
        match e {
            system_tray::client::Event::Add(dest, status_notifier_item) => {
                let item = Arc::new(Mutex::new(Tray::new(*status_notifier_item)));
                let dest = Arc::new(dest);

                self.add_tray(dest.clone(), item.clone());

                Some(TrayEventSignal::Add(dest, item))
            }
            system_tray::client::Event::Remove(id) => {
                let dest = Arc::new(id.clone());

                self.remove_tray(&dest);

                Some(TrayEventSignal::Rm(dest))
            }
            system_tray::client::Event::Update(id, update_event) => {
                let dest = Arc::new(id);

                let need_update = match update_event {
                    system_tray::client::UpdateEvent::Menu(tray_menu) => {
                        if let Some(tray) = self.get_mut(&dest) {
                            tray.lock()
                                .unwrap()
                                .update_menu(RootMenu::from_tray_menu(tray_menu))
                        }
                        true
                    }
                    system_tray::client::UpdateEvent::Title(title) => self
                        .get_mut(&dest)
                        .map(|tray| tray.lock().unwrap().update_title(title))
                        .unwrap_or_default(),
                    system_tray::client::UpdateEvent::Icon {
                        icon_name,
                        icon_pixmap,
                    } => {
                        let icon = icon_name
                            .filter(|name| !name.is_empty())
                            .map(Icon::Named)
                            .or_else(|| {
                                icon_pixmap
                                    .filter(|pixmap| !pixmap.is_empty())
                                    .map(Icon::Pixmap)
                            });

                        self.get_mut(&dest)
                            .map(|tray| tray.lock().unwrap().update_icon(icon))
                            .unwrap_or_default()
                    }

                    // not implemented
                    system_tray::client::UpdateEvent::AttentionIcon(_) => {
                        log::warn!("NOT IMPLEMENTED ATTENTION ICON");
                        false
                    }
                    system_tray::client::UpdateEvent::OverlayIcon(_) => {
                        log::warn!("NOT IMPLEMENTED OVERLAY ICON");
                        false
                    }
                    system_tray::client::UpdateEvent::Status(_) => {
                        // no need
                        log::warn!("NOT IMPLEMENTED STATUS");
                        false
                    }
                    system_tray::client::UpdateEvent::Tooltip(_) => {
                        // maybe some other time
                        log::warn!("NOT IMPLEMENTED TOOLTIP");
                        false
                    }
                    system_tray::client::UpdateEvent::MenuDiff(diffs) => {
                        if let Some(tray) = self.get_mut(&dest) {
                            diffs
                                .into_iter()
                                .for_each(|diff| tray.lock().unwrap().update_menu_item(diff));
                        }
                        true
                    }
                    system_tray::client::UpdateEvent::MenuConnect(_) => {
                        // no need i think?
                        log::warn!("NOT IMPLEMENTED MENU CONNECT");
                        false
                    }
                };

                if need_update {
                    Some(TrayEventSignal::Update(dest))
                } else {
                    None
                }
            }
        }
    }
}

// #[derive(Debug)]
// pub struct TrayMap {
//     pub(super) inner: HashMap<Arc<String>, Tray>,
// }
// impl TrayMap {
//     // Allow Arc<Mutex<TrayMap>> despite TrayMap not being Send+Sync due to cairo::ImageSurface.
//     // This is acceptable because:
//     // 1. Application uses single-threaded async runtime (LocalRuntime)
//     // 2. Arc<Mutex<...>> pattern required by Wayland API constraints (WlSurface::data needs Send+Sync)
//     // 3. Cairo surfaces are inherently not thread-safe and shouldn't cross thread boundaries
//     #[allow(clippy::arc_with_non_send_sync)]
//     fn new() -> Arc<Mutex<Self>> {
//         Arc::new(Mutex::new(Self {
//             inner: HashMap::new(),
//         }))
//     }
//     pub fn list_tray(&self) -> Vec<(Arc<String>, &Tray)> {
//         let a: Vec<_> = self.inner.iter().collect();
//         self.inner.iter().map(|(k, v)| (k.clone(), v)).collect()
//     }
//     pub fn get_tray<Q: Hash + Eq>(&self, destination: &Q) -> Option<&Tray>
//     where
//         Arc<String>: Borrow<Q>,
//     {
//         self.inner.get(destination)
//     }
// }

pub(super) struct TrayContext {
    pub client: Client,
    tray_map: TrayMap,
    cbs: HashMap<i32, Sender<TrayMsg>>,
    count: i32,
}
unsafe impl Send for TrayContext {}
unsafe impl Sync for TrayContext {}
impl TrayContext {
    fn new(client: Client) -> Self {
        Self {
            client,
            cbs: HashMap::new(),
            count: 0,
            tray_map: TrayMap::default(),
        }
    }
    pub fn call(&mut self, e: Event) {
        if let Some(dest) = self.tray_map.handle_event(e) {
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

static TRAY_CONTEXT: AtomicPtr<Arc<tokio::sync::Mutex<TrayContext>>> =
    AtomicPtr::new(std::ptr::null_mut());
pub(super) fn get_tray_context() -> Arc<tokio::sync::Mutex<TrayContext>> {
    unsafe {
        TRAY_CONTEXT
            .load(std::sync::atomic::Ordering::Acquire)
            .as_ref()
            .unwrap()
            .clone()
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
        Box::into_raw(Box::new(Arc::new(tokio::sync::Mutex::new(
            TrayContext::new(client),
        )))),
        std::sync::atomic::Ordering::Release,
    );
    CONTEXT_INITED.store(true, std::sync::atomic::Ordering::Release);

    get_backend_runtime_handle().spawn(async move {
        while let Ok(ev) = tray_rx.recv().await {
            // log::debug!("tray event: {ev:?}");
            get_tray_context().lock().await.call(ev);
        }
    });
}

pub fn register_tray(cb: Sender<TrayMsg>) -> TrayBackendHandle {
    get_tray_context().blocking_lock().add_cb(cb)
}

pub fn unregister_tray(id: i32) {
    get_tray_context().blocking_lock().remove_cb(id);
}
