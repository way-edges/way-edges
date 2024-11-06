use super::{
    calculate_config_relative, create_widgets, get_monitor_context, WidgetItem, WidgetMap,
};
use crate::config::GroupConfig;
use gtk::prelude::MonitorExt;

pub struct Default(WidgetMap);
impl Default {
    pub fn init_window(app: &gtk::Application, cfgs: GroupConfig) -> Result<Self, String> {
        let monitor_ctx = get_monitor_context();

        let btis: Vec<WidgetItem> = cfgs
            .into_iter()
            .map(|mut cfg| {
                let monitor = monitor_ctx
                    .get_monitor(&cfg.monitor)
                    .ok_or("failed to get monitor")?
                    .clone();
                let geom = monitor.geometry();
                let size = (geom.width(), geom.height());
                calculate_config_relative(&mut cfg, size)?;
                Ok(WidgetItem { cfg, monitor })
            })
            .collect::<Result<Vec<WidgetItem>, String>>()?;

        let widget_map = create_widgets(app, btis)?;

        Ok(Self(widget_map))
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
