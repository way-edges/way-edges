#![cfg(not(feature = "hyprland"))]

use super::{calculate_relative, create_buttons, find_monitor, ButtonItem};
use crate::config::GroupConfig;
use gtk::prelude::{ApplicationExt, MonitorExt};

pub struct Default;
impl super::WindowInitializer for Default {
    fn init_window(app: &gtk::Application, cfgs: GroupConfig) {
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
            create_buttons(app, btis);
            Ok(())
        });
        if let Err(e) = res {
            app.quit();
            super::notify_app_error(e)
        }
    }
}
