use gio::{prelude::*, ListModel};
use gtk::gdk::Monitor;
use gtk::prelude::{DisplayExt, MonitorExt};
use serde::Deserialize;
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicPtr};

use crate::notify_send;

use super::GroupMapCtxRc;

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum MonitorSpecifier {
    ID(usize),
    Name(String),
}
impl Default for MonitorSpecifier {
    fn default() -> Self {
        Self::ID(0)
    }
}

pub struct MonitorCtx {
    pub list_model: ListModel,
    pub monitors: Vec<Monitor>,
    pub name_index_map: HashMap<String, usize>,
}
impl MonitorCtx {
    fn new(list_model: ListModel) -> Self {
        Self {
            list_model,
            monitors: Vec::new(),
            name_index_map: HashMap::new(),
        }
    }

    pub fn get_monitor(&self, specifier: &MonitorSpecifier) -> Option<&Monitor> {
        match specifier {
            MonitorSpecifier::ID(index) => self.monitors.get(*index),
            MonitorSpecifier::Name(name) => self.monitors.get(*self.name_index_map.get(name)?),
        }
    }

    pub fn get_monitor_size(&self, specifier: &MonitorSpecifier) -> Option<(i32, i32)> {
        let monitor = self.get_monitor(specifier)?;
        let geom = monitor.geometry();
        Some((geom.width(), geom.height()))
    }

    fn reload_monitors(&mut self) -> Result<(), String> {
        self.monitors = self
            .list_model
            .iter::<Monitor>()
            .map(|m| m.map_err(|e| format!("Get monitor error: {e}")))
            .collect::<Result<Vec<Monitor>, String>>()?;

        self.name_index_map = self
            .monitors
            .iter()
            .enumerate()
            .map(|(index, monitor)| {
                let a = monitor
                    .connector()
                    .ok_or(format!("Fail to get monitor connector name: {monitor:?}"))?;
                Ok((a.to_string(), index))
            })
            .collect::<Result<HashMap<String, usize>, String>>()?;

        log::debug!(
            "Reloaded monitors: {:?}\n{:#?}",
            self.monitors,
            self.name_index_map
        );

        Ok(())
    }
}

pub static MONITORS: AtomicPtr<MonitorCtx> = AtomicPtr::new(std::ptr::null_mut());

pub fn get_monitor_context() -> &'static mut MonitorCtx {
    return unsafe {
        MONITORS
            .load(std::sync::atomic::Ordering::Acquire)
            .as_mut()
            .unwrap()
    };
}

pub fn init_monitor(group_map: GroupMapCtxRc) -> Result<(), String> {
    static IS_MONITOR_WATCHER_INITED: AtomicBool = AtomicBool::new(false);
    if IS_MONITOR_WATCHER_INITED.load(std::sync::atomic::Ordering::Acquire) {
        return Err("Monitor watcher already initialized".to_string());
    }

    let list_model = gtk::gdk::Display::default()
        .ok_or("display for monitor not found")?
        .monitors();

    let mut ctx = MonitorCtx::new(list_model.clone());
    ctx.reload_monitors()?;

    MONITORS.store(
        Box::into_raw(Box::new(ctx)),
        std::sync::atomic::Ordering::Release,
    );

    let debouncer_context: Cell<Option<Rc<Cell<bool>>>> = Cell::new(None);

    list_model.connect_items_changed(move |_, _, _, _| {
        println!("Monitor changed");
        use gtk::glib;

        if let Some(removed) = debouncer_context.take() {
            removed.set(true);
        }

        let removed = Rc::new(Cell::new(false));

        gtk::glib::timeout_add_seconds_local_once(
            5,
            glib::clone!(
                #[weak]
                group_map,
                #[weak]
                removed,
                move || {
                    if removed.get() {
                        return;
                    }

                    if let Err(e) = get_monitor_context().reload_monitors() {
                        let msg = format!("Fail to reload monitors: {e}");
                        log::error!("{msg}");
                        notify_send("Monitor Watcher", &msg, true);
                    }
                    group_map.borrow_mut().reload();
                }
            ),
        );
        debouncer_context.set(Some(removed.clone()))
    });

    Ok(())
}
