#![cfg(not(feature = "hyprland"))]

use super::{calculate_relative, create_buttons, find_monitor, ButtonItem};
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
        let res = super::get_monitors().and_then(|monitors| {
            let btis: Vec<ButtonItem> = cfgs
                .into_iter()
                .map(|mut cfg| {
                    let monitor = find_monitor(&monitors, cfg.monitor.clone())?;
                    let geom = monitor.geometry();
                    let size = (geom.width(), geom.height());
                    calculate_relative(&mut cfg, size)?;
                    Ok(ButtonItem { cfg, monitor })
                })
                .collect::<Result<Vec<ButtonItem>, String>>()?;
            // let vw = Rc::new(Cell::new(create_buttons(app, btis)?));
            let vw = create_buttons(app, btis)?;
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
