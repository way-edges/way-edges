use std::{
    collections::HashMap,
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicPtr},
};

use gtk::{gdk::Display, IconTheme};
use system_tray::client::Client;

use crate::get_main_runtime_handle;

use super::event::{match_event, TrayEvent};

type TrayCallback = Box<dyn FnMut(&TrayEvent)>;
pub(super) struct TrayContext {
    pub client: Client,
    icon_theme: IconTheme,
    cbs: HashMap<i32, TrayCallback>,
    count: i32,
}
impl TrayContext {
    fn new(client: Client) -> Self {
        // NOTE: INVESTIGATE LATER
        let display = Display::default().unwrap();
        let icon_theme = IconTheme::for_display(&display);
        Self {
            client,
            icon_theme,
            cbs: HashMap::new(),
            count: 0,
        }
    }
    pub fn get_icon_theme(&self) -> &IconTheme {
        &self.icon_theme
    }
    pub fn call(&mut self, e: TrayEvent) {
        self.cbs.iter_mut().for_each(|(_, cb)| cb(&e));
    }
    fn add_cb(&mut self, mut cb: TrayCallback) -> i32 {
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
                cb(&e);
                if let Some(menu) = menu {
                    let e = (id.clone(), super::event::Event::MenuNew(menu.clone()));
                    cb(&e);
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

    let client = get_main_runtime_handle().block_on(async { Client::new().await.unwrap() });
    let mut tray_rx = client.subscribe();

    TRAY_CONTEXT.store(
        Box::into_raw(Box::new(TrayContext::new(client))),
        std::sync::atomic::Ordering::Release,
    );
    CONTEXT_INITED.store(true, std::sync::atomic::Ordering::Release);

    use gtk::glib;

    glib::spawn_future_local(async move {
        while let Ok(ev) = tray_rx.recv().await {
            // println!("{ev:?}\n"); // do something with event...
            let e = match_event(ev);
            if let Some(e) = e {
                get_tray_context().call(e);
            }
        }
    });

    // get_main_runtime_handle().spawn();
}

pub fn register_tray(cb: impl FnMut(&TrayEvent) + 'static) -> i32 {
    get_tray_context().add_cb(Box::new(cb))
}

pub fn unregister_tray(id: i32) {
    get_tray_context().remove_cb(id);
}

pub fn tray_update_item_theme_search_path(theme: String) {
    let icon_theme = get_tray_context().get_icon_theme();
    if !icon_theme
        .search_path()
        .contains(&PathBuf::from(theme.clone()))
    {
        icon_theme.add_search_path(theme);
    }
}
