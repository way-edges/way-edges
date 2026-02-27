use cairo::ImageSurface;

use config::def::widgets::wrapbox::text::TextConfig;
use cosmic_text::{Color, FamilyOwned};
use util::text::draw_text;

#[derive(Debug)]
pub struct TextDrawer {
    pub fg_color: Color,
    pub font_family: FamilyOwned,
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
    pub fn draw_text(&self, text: &str) -> ImageSurface {
        let text_conf = util::text::TextConfig::new(
            self.font_family.as_family(),
            None,
            self.fg_color,
            self.font_pixel_size,
        );

        draw_text(text, text_conf).to_image_surface()
    }
}
