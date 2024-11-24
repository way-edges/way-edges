use std::{collections::HashMap, ops::Deref, sync::atomic::AtomicPtr};

use system_tray::client::{Client, Event};

use crate::{get_main_runtime_handle, ui::draws::util::ImageData};

pub struct TrayItem {
    pub id: String,
    pub title: Option<String>,
    pub icon: Option<ImageData>,
    pub menu_path: Option<String>,

    pub menu_id: i32,
    pub menus: Vec<Menu>,
}

pub struct Menu {
    id: i32,
    label: Option<String>,
    enabled: bool,
    icon: Option<ImageData>,
    menu_type: MenuType,
}

pub enum MenuType {
    Radio(bool),
    Check(bool),
    Parent(Vec<Menu>),
    Normal,
}

struct TrayContext {
    client: Client,
    cbs: HashMap<i32, Box<dyn FnMut(Event)>>,
    items: HashMap<String, TrayItem>,
}

static TRAY_CONTEXT: AtomicPtr<TrayContext> = AtomicPtr::new(std::ptr::null_mut());
pub fn get_tray_context() -> &'static mut TrayContext {
    unsafe {
        TRAY_CONTEXT
            .load(std::sync::atomic::Ordering::Acquire)
            .as_mut()
            .unwrap()
    }
}

fn match_event(e: Event) {
    match e {
        Event::Add(id, status_notifier_item) => todo!(),
        Event::Update(id, update_event) => todo!(),
        Event::Remove(id) => todo!(),
    }
}

pub fn init_tray_client() {
    let client = get_main_runtime_handle().block_on(async { Client::new().await.unwrap() });
    let mut tray_rx = client.subscribe();

    {
        let initial_items = client.items();
        let a = initial_items.lock().unwrap();
        println!("Initial items: {:?}\n", a.deref());
    }

    // get_main_runtime_handle().spawn(async {
    //     // do something with initial items...
    //
    //     while let Ok(ev) = tray_rx.recv().await {
    //         println!("{ev:?}\n"); // do something with event...
    //     }
    // });
}
