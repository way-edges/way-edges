use super::{calculate_height, create_buttons, find_monitor, ButtonItem};
use crate::config::GroupConfig;
use gtk::prelude::MonitorExt;

pub struct Default;
impl super::WindowInitializer for Default {
    fn init_window(app: &gtk::Application, cfgs: GroupConfig) {
        let monitors = super::get_monitors();
        let btis: Vec<ButtonItem> = cfgs
            .into_iter()
            .map(|mut cfg| {
                let monitor = find_monitor(&monitors, cfg.monitor.clone());
                if cfg.rel_height > 0. {
                    let geom = monitor.geometry();
                    calculate_height(&mut cfg, (geom.width(), geom.height()));
                };
                ButtonItem { cfg, monitor }
            })
            .collect();
        create_buttons(app, btis);
    }
}
