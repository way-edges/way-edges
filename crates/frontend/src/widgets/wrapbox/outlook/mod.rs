use cairo::ImageSurface;
use config::{widgets::wrapbox::Outlook, Config};

pub mod window;

pub trait OutlookMousePositionTranslateion {
    fn translate_mouse_position(&self, pos: (f64, f64)) -> (f64, f64);
}

pub fn init_outlook(
    outlook: Outlook,
    conf: &Config,
) -> (
    impl OutlookMousePositionTranslateion,
    impl Fn(ImageSurface) -> ImageSurface,
) {
    match outlook {
        Outlook::Window(outlook_window_config) => {
            window::make_draw_func(outlook_window_config, conf.edge)
        }
    }
}
