use std::fmt::Debug;

use cairo::ImageSurface;
use config::def::{shared::NumOrRelative, widgets::wrapbox::Outlook};
use smithay_client_toolkit::shell::wlr_layer::Anchor;

mod board;
mod window;

pub fn init_outlook(outlook: &Outlook, edge: Anchor, offset: NumOrRelative) -> Box<dyn OutlookDraw> {
    match outlook {
        Outlook::Window(outlook_window_config) => {
            Box::new(window::DrawConf::new(outlook_window_config, edge, offset))
        }
        Outlook::Board(outlook_board_config) => {
            Box::new(board::DrawConf::new(outlook_board_config, edge, offset))
        }
    }
}

pub trait OutlookDraw: Debug {
    fn draw(&mut self, content: ImageSurface) -> ImageSurface;
    fn translate_mouse_position(&self, pos: (f64, f64)) -> (f64, f64);
}
