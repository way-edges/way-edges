pub mod default;

#[cfg(feature = "hyprland")]
pub mod hyprland;

use std::collections::HashMap;

use crate::config::Config;
use crate::ui::{self, WidgetCtx};
use gtk::gdk::Monitor;
use gtk::prelude::GtkWindowExt;
use gtk4_layer_shell::{Edge, LayerShell};

fn notify_app_error(err_des: &str) {
    log::error!("{err_des}");
    crate::notify_send("Way-edges app error", err_des, true);
}

fn find_monitor<'a>(
    monitors: &'a [Monitor],
    specifier: &MonitorSpecifier,
) -> Result<&'a Monitor, String> {
    let index = match specifier {
        MonitorSpecifier::ID(index) => *index,
        MonitorSpecifier::Name(name) => get_monitor_index_by_name(name)?,
    };
    monitors
        .get(index)
        .ok_or(format!("error matching monitor with id: {index}"))
}

fn calculate_config_relative(cfg: &mut Config, max_size_raw: (i32, i32)) -> Result<(), String> {
    cfg.margins.iter_mut().for_each(|(e, n)| {
        match e {
            Edge::Left | Edge::Right => n.calculate_relative(max_size_raw.0 as f64),
            Edge::Top | Edge::Bottom => n.calculate_relative(max_size_raw.1 as f64),
            _ => unreachable!(),
        };
    });
    Ok(())
}

pub trait GroupCtx {
    fn close(&mut self);
    fn widget_map(&mut self) -> &mut WidgetMap;
}

struct WidgetItem {
    cfg: Config,
    monitor: Monitor,
}

pub type WidgetMap = HashMap<String, WidgetCtx>;

fn create_widgets(
    app: &gtk::Application,
    widget_items: Vec<WidgetItem>,
) -> Result<WidgetMap, String> {
    let a = widget_items
        .into_iter()
        .map(|w| {
            let key = w.cfg.name.clone();
            let widget_ctx = ui::new_window(app, w.cfg, &w.monitor)?;
            {
                let win = widget_ctx.window.upgrade().unwrap();
                win.set_namespace("way-edges-widget");
                win.present();
            }
            Ok((key, widget_ctx))
        })
        .collect::<Result<WidgetMap, String>>()?;
    Ok(a)
}

pub use globals::*;

mod globals {
    use gio::prelude::*;
    use gtk::gdk::{Monitor, Rectangle};
    use gtk::prelude::{DisplayExt, MonitorExt};
    use std::collections::HashMap;

    #[derive(Debug, Clone)]
    pub enum MonitorSpecifier {
        ID(usize),
        Name(String),
    }
    impl MonitorSpecifier {
        pub fn to_index(&self) -> Result<usize, String> {
            let index = match self {
                Self::ID(index) => *index,
                Self::Name(name) => get_monitor_index_by_name(name)?,
            };
            Ok(index)
        }
    }

    pub static mut MONITORS: Option<Vec<Monitor>> = None;
    // pub fn init_monitor() -> Result<&'static Vec<Monitor>, String> {
    pub fn init_monitor() -> Result<(), String> {
        let dt_display = gtk::gdk::Display::default().ok_or("display for monitor not found")?;
        let mms = dt_display
            .monitors()
            .iter::<Monitor>()
            .map(|m| m.map_err(|e| format!("Set monitor error: {e}")))
            .collect::<Result<Vec<Monitor>, String>>()?;
        {
            let name_index_map = mms
                .iter()
                .enumerate()
                .map(|(index, m)| {
                    let a = m
                        .connector()
                        .ok_or(format!("Fail to get monitor connector name: {m:?}"))?;
                    Ok((a.to_string(), index))
                })
                .collect::<Result<MonitorNameIndexMap, String>>()?;
            set_monitor_name_index(name_index_map);
        }
        log::debug!("Set monitors: {mms:?}");
        {
            let geoms: Vec<Rectangle> = mms.iter().map(|m| m.geometry()).collect();
            set_monitor_size_map(geoms);
        }
        unsafe {
            WORKING_AREA_SIZE_MAP = Some(HashMap::new());
        }
        unsafe { MONITORS = Some(mms) };
        Ok(())
        // get_monitors()
    }
    pub fn get_monitors() -> Result<&'static Vec<Monitor>, String> {
        unsafe { MONITORS.as_ref().ok_or("MONITORS is NONE".to_string()) }
    }
    // pub fn take_monitor() -> Result<Vec<Monitor>, String> {
    //     unsafe { MONITORS.take().ok_or("MONITORS is NONE".to_string()) }
    // }

    pub type MonitorNameIndexMap = HashMap<String, usize>;
    pub static mut MONITOR_NAME_INDEX_MAP: Option<MonitorNameIndexMap> = None;
    pub fn set_monitor_name_index(map: MonitorNameIndexMap) {
        unsafe { MONITOR_NAME_INDEX_MAP = Some(map) }
    }
    pub fn get_monitor_index_by_name(name: &str) -> Result<usize, String> {
        unsafe {
            let map = MONITOR_NAME_INDEX_MAP
                .as_ref()
                .ok_or("MONITOR_NAME_INDEX_MAP has not been initialized")?;
            map.get(name).copied().ok_or("Name not found".to_string())
        }
    }

    pub type Size = (i32, i32);

    // pub static mut MONITOR_SIZE_MAP: Option<Vec<(Monitor, (i32, i32))>> = None;
    /// working area size
    pub static mut WORKING_AREA_SIZE_MAP: Option<HashMap<usize, Rectangle>> = None;
    // do not run this directly, unless you know what you are doing
    fn get_working_area_size_map() -> Result<&'static mut HashMap<usize, Rectangle>, String> {
        unsafe {
            WORKING_AREA_SIZE_MAP
                .as_mut()
                .ok_or("MONITOR_SIZE_MAP has not been initialized, this is unexpected".to_string())
        }
    }
    pub fn set_working_area_size_map(index: usize, v: Rectangle) -> Result<(), String> {
        let map = get_working_area_size_map()?;
        map.insert(index, v);
        Ok(())
    }
    pub fn set_working_area_size_map_multiple(v: Vec<(usize, Rectangle)>) -> Result<(), String> {
        let map = get_working_area_size_map()?;
        for (i, r) in v.into_iter() {
            map.insert(i, r);
        }
        log::debug!("Calculated layer map sizes: {map:?}");
        Ok(())
    }
    pub fn get_working_area_size(index: usize) -> Result<Option<Size>, String> {
        unsafe {
            let map = WORKING_AREA_SIZE_MAP
                .as_ref()
                .ok_or("WORKING_AREA_SIZE_MAP has not been initialized")?;
            if let Some(geom) = map.get(&index) {
                Ok(Some((geom.width(), geom.height())))
            } else {
                Ok(None)
            }
        }
    }

    /// monitor size
    pub static mut MONITOR_SIZE_MAP: Option<HashMap<usize, Rectangle>> = None;
    pub fn set_monitor_size_map(geoms: Vec<Rectangle>) {
        unsafe {
            MONITOR_SIZE_MAP = Some(HashMap::from_iter(geoms.into_iter().enumerate()));
        }
    }
    pub fn get_monior_size(index: usize) -> Result<Option<Size>, String> {
        unsafe {
            let map = MONITOR_SIZE_MAP
                .as_ref()
                .ok_or("MONITOR_SIZE_MAP has not been initialized")?;
            if let Some(geom) = map.get(&index) {
                Ok(Some((geom.width(), geom.height())))
            } else {
                Ok(None)
            }
        }
    }
}
