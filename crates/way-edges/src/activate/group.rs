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
    fn get_app(&self) -> Option<Application> {
        self.app.as_ref().and_then(|weak| weak.upgrade())
    }
    pub fn add_group(&mut self, name: &str) {
        if self.map.contains_key(name) {
            return;
        }
        let widgets_or_not = self
            .get_app()
            .and_then(|app| GroupMapCtx::init_group(&app, name).ok());
        self.map.insert(name.to_string(), widgets_or_not);
    }
    pub fn rm_group(&mut self, name: &str) {
        drop(self.map.remove(name));
    }
    pub fn reload(&mut self) {
        if let Some(app) = self.get_app() {
            self.map.iter_mut().for_each(|(k, widget_map)| {
                drop(widget_map.take());
                *widget_map = GroupMapCtx::init_group(&app, k.as_str()).ok();
            });
        }
    }
    pub fn dispose(&mut self) {
        self.map.clear();
        if let Some(app) = self.get_app() {
            app.quit()
        }
        drop(self.hold.take());
    }
    pub fn toggle_pin(&mut self, gn: &str, wn: &str) {
        let Some(Some(widgets)) = self.map.get_mut(gn) else {
            return;
        };
        if let Some(w) = widgets.get_widget(wn) {
            w.toggle_pin()
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
