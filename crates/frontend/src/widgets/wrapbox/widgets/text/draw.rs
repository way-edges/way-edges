use cairo::ImageSurface;

use config::widgets::wrapbox::text::TextConfig;
use cosmic_text::Color;
use util::text::draw_text;

use super::TextCtx;
use crate::widgets::wrapbox::box_traits::BoxedWidget;

#[derive(Debug)]
pub struct TextDrawer {
    pub fg_color: Color,
    pub font_family: Option<String>,
    pub font_pixel_size: i32,
}
impl TextDrawer {
    pub fn new(conf: &TextConfig) -> Self {
        Self {
            fg_color: conf.fg_color,
            font_family: conf.font_family.clone(),
            font_pixel_size: conf.font_size,
        }
    }
    fn draw_text(&self, text: &str) -> ImageSurface {
        let text_conf = util::text::TextConfig::new(
            self.font_family.as_deref(),
            None,
            self.fg_color,
            self.font_pixel_size,
        );

        draw_text(text, text_conf).to_image_surface()
    }
}

impl BoxedWidget for TextCtx {
    fn content(&mut self) -> ImageSurface {
        let text = unsafe { self.text.get().as_ref().unwrap().as_str() };
        self.drawer.draw_text(text)
    }
}
