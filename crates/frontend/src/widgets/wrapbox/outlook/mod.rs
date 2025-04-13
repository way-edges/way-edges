use std::fmt::Debug;

use cairo::ImageSurface;
use config::{widgets::wrapbox::Outlook, Config};

mod board;
mod window;

pub fn init_outlook(outlook: Outlook, conf: &Config) -> Box<dyn OutlookDraw> {
    match outlook {
        Outlook::Window(outlook_window_config) => {
            Box::new(window::DrawConf::new(outlook_window_config, conf.edge))
        }
        Outlook::Board(outlook_board_config) => {
            Box::new(board::DrawConf::new(outlook_board_config, conf.edge))
        }
    }
}

pub trait OutlookDraw: Debug {
    fn draw(&mut self, content: ImageSurface) -> ImageSurface;
    fn translate_mouse_position(&self, pos: (f64, f64)) -> (f64, f64);
}
