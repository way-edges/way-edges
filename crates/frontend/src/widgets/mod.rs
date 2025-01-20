use cairo::ImageSurface;

use crate::{
    mouse_state::{MouseEvent, MouseStateData},
    wayland::app::WidgetBuilder,
};

mod button;
mod hypr_workspace;
mod slide;
mod wrapbox;

pub trait WidgetContext {
    fn redraw(&mut self) -> ImageSurface;
    fn on_mouse_event(&mut self, data: &MouseStateData, event: MouseEvent) -> bool;
}

pub fn init_widget(
    conf: &mut config::Config,
    builder: &mut WidgetBuilder,
) -> Box<dyn WidgetContext> {
    let monitor = builder.app.output_state.info(&builder.output).unwrap();
    let size = monitor.modes[0].dimensions;

    match conf.widget.take().unwrap() {
        config::widgets::Widget::Btn(btn_config) => {
            log::debug!("initializing button");
            let w = button::init_widget(builder, size, conf, btn_config);
            log::info!("initialized button");
            Box::new(w)
        }
        config::widgets::Widget::Slider(slide_config) => {
            log::debug!("initializing slider");
            let w = slide::init_widget(builder, size, conf, slide_config);
            log::info!("initialized slider");
            w
        }
        config::widgets::Widget::HyprWorkspace(hypr_workspace_config) => {
            log::debug!("initializing hypr-workspace");
            let w = hypr_workspace::init_widget(builder, size, conf, hypr_workspace_config);
            log::info!("initialized hypr-workspace");
            Box::new(w)
        }
        config::widgets::Widget::WrapBox(box_config) => {
            log::debug!("initializing box");
            let w = wrapbox::init_widget(builder, conf, box_config);
            log::info!("initialized box");
            Box::new(w)
        }
    }
}
