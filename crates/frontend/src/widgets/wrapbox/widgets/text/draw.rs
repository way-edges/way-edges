use cairo::ImageSurface;
use gdk::RGBA;

use config::widgets::wrapbox::text::TextConfig;
use util::{rgba_to_color, text::draw_text};

use super::TextCtx;
use crate::widgets::wrapbox::box_traits::BoxedWidget;

#[derive(Debug)]
pub struct TextDrawer {
    pub fg_color: RGBA,
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
        // let layout = {
        //     let pc = pangocairo::pango::Context::new();
        //     let fm = pangocairo::FontMap::default();
        //     pc.set_font_map(Some(&fm));
        //
        //     let mut desc = pc.font_description().unwrap();
        //     desc.set_size(self.font_pixel_size << 10);
        //     if let Some(font_family) = self.font_family.as_ref() {
        //         desc.set_family(font_family.as_str());
        //     }
        //     pc.set_font_description(Some(&desc));
        //     pangocairo::pango::Layout::new(&pc)
        // };

        let text_conf = util::text::TextConfig::new(
            self.font_family.as_deref(),
            rgba_to_color(self.fg_color),
            self.font_pixel_size,
        );

        draw_text(&text, text_conf).to_image_surface()

        // draw_text_to_size(&layout, &self.fg_color, text, self.font_pixel_size)
    }
}

impl BoxedWidget for TextCtx {
    fn content(&mut self) -> ImageSurface {
        let text = unsafe { self.text.get().as_ref().unwrap().as_str() };
        self.drawer.draw_text(text)
    }
}
