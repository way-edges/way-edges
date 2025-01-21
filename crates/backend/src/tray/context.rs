use std::{
    collections::HashMap,
    rc::Rc,
    sync::atomic::{AtomicBool, AtomicPtr},
};

use calloop::channel::Sender;
use system_tray::client::Client;

use crate::runtime::get_backend_runtime_handle;

use super::event::{match_event, TrayEvent};

pub(super) struct TrayContext {
    pub client: Client,
    cbs: HashMap<i32, Sender<Rc<TrayEvent>>>,
    count: i32,
}
impl TrayContext {
    fn new(client: Client) -> Self {
        Self {
            client,
            cbs: HashMap::new(),
            count: 0,
        }
    }
    pub fn call(&mut self, e: TrayEvent) {
        let e = Rc::new(e);
        self.cbs.iter().for_each(|(_, cb)| {
            cb.send(e.clone()).unwrap();
        });
    }
    fn add_cb(&mut self, cb: Sender<Rc<TrayEvent>>) -> i32 {
        let key = self.count;
        self.count += 1;

        self.client
            .items()
            .lock()
            .unwrap()
            .iter()
            .for_each(|(id, (item, menu))| {
                let e = (
                    id.clone(),
                    super::event::Event::ItemNew(item.clone().into()),
                );
                cb.send(Rc::new(e)).unwrap();
                if let Some(menu) = menu {
                    let e = (id.clone(), super::event::Event::MenuNew(menu.clone()));
                    cb.send(Rc::new(e)).unwrap();
                }
            });

        self.cbs.insert(key, cb);
        key
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
            let e = match_event(ev);
            if let Some(e) = e {
                get_tray_context().call(e);
            }
        }
    });

    // get_main_runtime_handle().spawn();
}

pub fn register_tray(cb: Sender<Rc<TrayEvent>>) -> i32 {
    get_tray_context().add_cb(cb)
}

pub fn unregister_tray(id: i32) {
    get_tray_context().remove_cb(id);
}

// pub fn tray_update_item_theme_search_path(theme: String) {
//     let icon_theme = get_tray_context().get_icon_theme();
//     if !icon_theme
//         .search_path()
//         .contains(&PathBuf::from(theme.clone()))
//     {
//         icon_theme.add_search_path(theme);
//     }
// }
