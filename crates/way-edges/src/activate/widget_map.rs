use config::GroupConfig;
use frontend::window::WindowContext;
use std::collections::HashMap;

pub struct WidgetMap(HashMap<String, WindowContext>);

impl WidgetMap {
    pub fn init_window(app: &gtk::Application, cfgs: GroupConfig) -> Result<Self, String> {
        let widget_map = cfgs
            .into_iter()
            .map(|cfg| {
                let widget_name = cfg.name.clone();
                let window_ctx = frontend::widgets::init_widget(app, cfg)?;
                Ok((widget_name, window_ctx))
            })
            .collect::<Result<HashMap<String, WindowContext>, String>>()?;

        Ok(Self(widget_map))
    }

    pub fn get_widget(&mut self, name: &str) -> Option<&mut WindowContext> {
        self.0.get_mut(name)
    }
}
