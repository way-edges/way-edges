use super::{
    calculate_config_relative, create_widgets, find_monitor, get_monitors, WidgetItem, WidgetMap,
};
use crate::config::GroupConfig;
use gtk::prelude::MonitorExt;

pub struct Default(WidgetMap);
impl Default {
    pub fn init_window(app: &gtk::Application, cfgs: GroupConfig) -> Result<Self, String> {
        let res = get_monitors().and_then(|monitors| {
            let btis: Vec<WidgetItem> = cfgs
                .into_iter()
                .map(|mut cfg| {
                    let monitor = find_monitor(monitors, &cfg.monitor)?.clone();
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
impl Drop for Default {
    fn drop(&mut self) {
        self.0.iter_mut().for_each(|(_, v)| v.close())
    }
}

impl super::GroupCtx for Default {
    fn close(&mut self) {
        self.0.iter_mut().for_each(|(_, v)| v.close());
    }

    fn widget_map(&mut self) -> &mut WidgetMap {
        &mut self.0
    }
}
