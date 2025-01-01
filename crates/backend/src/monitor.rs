use gio::{prelude::*, ListModel};
use gtk::gdk::Monitor;
use gtk::prelude::{DisplayExt, MonitorExt};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicPtr};

use config::MonitorSpecifier;

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

    pub fn reload_monitors(&mut self) -> Result<(), String> {
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

        log::info!(
            "Reloaded monitors:\n{:?}\n{:?}",
            self.monitors,
            self.name_index_map
        );

        Ok(())
    }
}

static MONITORS: AtomicPtr<MonitorCtx> = AtomicPtr::new(std::ptr::null_mut());

pub fn get_monitor_context() -> &'static mut MonitorCtx {
    unsafe {
        MONITORS
            .load(std::sync::atomic::Ordering::Acquire)
            .as_mut()
            .unwrap()
    }
}

pub fn init_monitor(cb: impl Fn(&ListModel, u32, u32, u32) + 'static) -> Result<(), String> {
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

    list_model.connect_items_changed(cb);

    Ok(())
}
