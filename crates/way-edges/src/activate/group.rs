use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gio::{
    glib::{clone::Downgrade, WeakRef},
    prelude::{ApplicationExt, ApplicationExtManual},
    ApplicationHoldGuard,
};
use gtk::Application;
use log::debug;

use super::widget_map::WidgetMap;

pub struct GroupMapCtx {
    pub map: HashMap<String, Option<WidgetMap>>,
    pub app: Option<WeakRef<Application>>,

    // keep reference alive
    pub hold: Option<ApplicationHoldGuard>,
}
impl GroupMapCtx {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            app: None,
            hold: None,
        }
    }
    pub fn init_with_app(&mut self, app: &Application) {
        self.hold = Some(app.hold());
        self.app = Some(Downgrade::downgrade(app));
    }
    pub fn add_group(&mut self, name: &str) {
        if !self.map.contains_key(name) {
            if let Some(app) = &self.app {
                let s = GroupMapCtx::init_group(&app.upgrade().unwrap(), name).ok();
                self.map.insert(name.to_string(), s);
            } else {
                self.map.insert(name.to_string(), None);
            }
        }
    }
    pub fn rm_group(&mut self, name: &str) {
        if let Some(Some(mut widget_map)) = self.map.remove(name) {
            widget_map.close()
        }
    }
    pub fn reload(&mut self) {
        if let Some(app) = &self.app {
            let app = app.upgrade().unwrap();
            self.map.iter_mut().for_each(|(k, widget_map)| {
                if let Some(mut widget_map) = widget_map.take() {
                    widget_map.close()
                }
                *widget_map = GroupMapCtx::init_group(&app, k.as_str()).ok();
            });
        }
    }
    pub fn dispose(&mut self) {
        self.map.iter_mut().for_each(|(_, v)| {
            if let Some(widget_map) = v.as_mut() {
                widget_map.close()
            }
        });
        if let Some(app) = &self.app {
            if let Some(app) = app.upgrade() {
                app.quit()
            }
        }
        drop(self.hold.take());
    }
    pub fn toggle_pin(&mut self, gn: &str, wn: &str) {
        if let Some(Some(v)) = self.map.get_mut(gn) {
            if let Some(v) = v.get_widget(wn) {
                v.widget_expose.toggle_pin()
            }
        }
    }

    fn init_group(app: &Application, name: &str) -> Result<WidgetMap, String> {
        let conf = config::get_config_by_group(Some(name));
        let res = conf.and_then(|vc| {
            let Some(vc) = vc else {
                return Err(format!("Not found config by group: {name}"));
            };
            debug!("Parsed Config: {vc:?}");
            WidgetMap::init_window(app, vc.widgets)
        });
        res.inspect_err(|e| {
            log::error!("{e}");
            util::notify_send("Way-edges app error", e, true);
        })
    }
}

pub type GroupMapCtxRc = Rc<RefCell<GroupMapCtx>>;
