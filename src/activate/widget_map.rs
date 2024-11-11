use gtk::prelude::{GtkWindowExt, MonitorExt};
use gtk4_layer_shell::{Edge, LayerShell};

use crate::activate::monitor;
use crate::config::{Config, GroupConfig};
use crate::ui::{self, WidgetCtx};

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

type _WidgetHashMap = Vec<(String, WidgetCtx)>;

pub struct WidgetMap(_WidgetHashMap);

impl WidgetMap {
    pub fn init_window(app: &gtk::Application, cfgs: GroupConfig) -> Result<Self, String> {
        let monitor_ctx = monitor::get_monitor_context();

        let widget_map = cfgs
            .into_iter()
            .map(|mut cfg| {
                // get monitor and calculate size
                let monitor = monitor_ctx
                    .get_monitor(&cfg.monitor)
                    .ok_or("failed to get monitor")?
                    .clone();
                let geom = monitor.geometry();
                calculate_config_relative(&mut cfg, (geom.width(), geom.height()))?;

                // create widget and present
                let widget_name = cfg.name.clone();
                let widget_ctx = ui::new_window(app, cfg, &monitor)?;
                let window = widget_ctx.window.upgrade().unwrap();
                window.set_namespace("way-edges-widget");
                window.present();

                // return widget name and widget context
                Ok((widget_name, widget_ctx))
            })
            .collect::<Result<_WidgetHashMap, String>>()?;

        Ok(Self(widget_map))
    }

    pub fn close(&mut self) {
        self.0.iter_mut().for_each(|(_, v)| v.close());
    }

    pub fn get_widget(&mut self, name: &str) -> Option<&mut WidgetCtx> {
        self.0
            .iter_mut()
            .find(|(widget_name, _)| name == widget_name)
            .map(|(_, widget)| widget)
    }
}
impl Drop for WidgetMap {
    fn drop(&mut self) {
        self.close();
    }
}
