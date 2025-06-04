use cairo::ImageSurface;

use crate::{
    mouse_state::{MouseEvent, MouseStateData},
    wayland::app::WidgetBuilder,
};

mod button;
mod slide;
mod workspace;
mod wrapbox;

pub trait WidgetContext: std::fmt::Debug {
    fn redraw(&mut self) -> ImageSurface;
    fn on_mouse_event(&mut self, data: &MouseStateData, event: MouseEvent) -> bool;
}

pub fn init_widget(
    conf: &mut config::Widget,
    builder: &mut WidgetBuilder,
) -> Box<dyn WidgetContext> {
    let monitor = builder.app.output_state.info(&builder.output).unwrap();
    let size = monitor.modes[0].dimensions;

    match conf {
        config::widgets::Widget::Btn(btn_config) => {
            log::debug!("initializing button");
            let w = button::init_widget(builder, size, btn_config);
            log::info!("initialized button");
            Box::new(w)
        }
        config::widgets::Widget::Slider(slide_config) => {
            log::debug!("initializing slider");
            let w = slide::init_widget(builder, size, slide_config);
            log::info!("initialized slider");
            w
        }
        config::widgets::Widget::Workspace(workspace_config) => {
            log::debug!("initializing workspace");
            let w = workspace::init_widget(builder, size, workspace_config, &monitor);
            log::info!("initialized workspace");
            Box::new(w)
        }
        config::widgets::Widget::WrapBox(box_config) => {
            log::debug!("initializing box");
            let w = wrapbox::init_widget(builder, box_config);
            log::info!("initialized box");
            Box::new(w)
        }
    }
}
