use cairo::ImageSurface;
use config::{widgets::wrapbox::Outlook, Config};

pub mod window;

pub fn init_outlook(outlook: Outlook, conf: &Config) -> OutlookDrawConf {
    match outlook {
        Outlook::Window(outlook_window_config) => {
            OutlookDrawConf::Window(window::DrawConf::new(outlook_window_config, conf.edge))
        }
    }
}

pub enum OutlookDrawConf {
    Window(window::DrawConf),
}

impl OutlookDrawConf {
    pub fn draw(&mut self, content: ImageSurface) -> ImageSurface {
        match self {
            OutlookDrawConf::Window(draw_conf) => draw_conf.draw(content),
        }
    }
    pub fn translate_mouse_position(&self, pos: (f64, f64)) -> (f64, f64) {
        match self {
            OutlookDrawConf::Window(draw_conf) => draw_conf.translate_mouse_position(pos),
        }
    }
}
