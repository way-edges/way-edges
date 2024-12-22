use config::GroupConfig;
use frontend::window::WindowContext;
use std::collections::HashMap;

type WidgetHashMap = HashMap<String, WindowContext>;

pub struct WidgetMap(WidgetHashMap);

impl WidgetMap {
    pub fn init_window(app: &gtk::Application, cfgs: GroupConfig) -> Result<Self, String> {
        let widget_map = cfgs
            .into_iter()
            .map(|cfg| {
                let widget_name = cfg.name.clone();
                let window_ctx = frontend::widgets::init_widget(app, cfg)?;
                Ok((widget_name, window_ctx))
            })
            .collect::<Result<WidgetHashMap, String>>()?;

        Ok(Self(widget_map))
    }

    pub fn get_widget(&mut self, name: &str) -> Option<&mut WindowContext> {
        self.0.get_mut(name)
    }
}
