#![cfg(not(feature = "hyprland"))]

use super::{calculate_config_relative, create_widgets, find_monitor, take_monitor, WidgetItem};
use crate::config::GroupConfig;
use gtk::{
    prelude::{GtkWindowExt, MonitorExt},
    ApplicationWindow,
};

#[derive(Clone)]
// pub struct Default(Rc<Cell<Vec<ApplicationWindow>>>);
pub struct Default(Vec<ApplicationWindow>);
impl super::WindowInitializer for Default {
    fn init_window(app: &gtk::Application, cfgs: GroupConfig) -> Result<Self, String> {
        let res = take_monitor().and_then(|monitors| {
            let btis: Vec<WidgetItem> = cfgs
                .into_iter()
                .map(|mut cfg| {
                    let monitor = find_monitor(&monitors, &cfg.monitor)?.clone();
                    let geom = monitor.geometry();
                    let size = (geom.width(), geom.height());
                    calculate_config_relative(&mut cfg, size)?;
                    Ok(WidgetItem { cfg, monitor })
                })
                .collect::<Result<Vec<WidgetItem>, String>>()?;
            let vw = create_widgets(app, btis)?;
            Ok(Self(vw))
        });
        res.inspect_err(|e| {
            super::notify_app_error(e);
        })
    }
}
impl super::WindowDestroyer for Default {
    fn close_window(self) {
        self.0.into_iter().for_each(|w| w.close());
    }
}
