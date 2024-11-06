pub mod default;

use std::collections::HashMap;

use crate::config::Config;
use crate::ui::{self, WidgetCtx};
use gtk::gdk::Monitor;
use gtk::prelude::GtkWindowExt;
use gtk4_layer_shell::{Edge, LayerShell};

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
    use gtk::gdk::Monitor;
    use gtk::prelude::{DisplayExt, MonitorExt};
    use std::collections::HashMap;
    use std::sync::atomic::AtomicPtr;

    #[derive(Debug, Clone)]
    pub enum MonitorSpecifier {
        ID(usize),
        Name(String),
    }

    pub struct MonitorCtx {
        pub monitors: Vec<Monitor>,
        pub name_index_map: HashMap<String, usize>,
    }
    impl MonitorCtx {
        fn new() -> Self {
            Self {
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
            let default_display =
                gtk::gdk::Display::default().ok_or("display for monitor not found")?;

            self.monitors = default_display
                .monitors()
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

    pub fn init_monitor() -> Result<(), String> {
        let mut ctx = MonitorCtx::new();
        ctx.reload_monitors()?;

        MONITORS.store(
            Box::into_raw(Box::new(ctx)),
            std::sync::atomic::Ordering::Release,
        );

        Ok(())
    }
}
