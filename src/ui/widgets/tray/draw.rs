use std::f64::consts::PI;

use cairo::{Context, ImageSurface};
use gtk::{gdk::RGBA, pango::Layout, prelude::GdkCairoContextExt};

use crate::ui::draws::util::{draw_text, draw_text_to_size, new_surface, Z};

use super::module::{MenuItem, MenuState, MenuType};

pub struct MenuDrawConfig {
    margin: [i32; 4],
    font_pixel_height: i32,
    marker_size: i32,
    separator_height: i32,
    text_color: RGBA,
    marker_color: Option<RGBA>,
}
impl Default for MenuDrawConfig {
    fn default() -> Self {
        Self {
            margin: [5; 4],
            marker_size: 16,
            font_pixel_height: 16,
            separator_height: 5,
            text_color: RGBA::BLACK,
            marker_color: None,
        }
    }
}

pub struct MenuDrawArg {
    draw_config: &'static MenuDrawConfig,
    layout: Layout,
}
impl MenuDrawArg {
    pub fn create_from_config(draw_config: &'static MenuDrawConfig) -> Self {
        let layout = {
            let font_size = draw_config.font_pixel_height;
            let pc = pangocairo::pango::Context::new();
            let mut desc = pc.font_description().unwrap();
            desc.set_absolute_size(font_size as f64 * 1024.);
            pc.set_font_description(Some(&desc));
            pangocairo::pango::Layout::new(&pc)
        };

        Self {
            draw_config,
            layout,
        }
    }

    pub fn draw_text(&self, text: &str) -> ImageSurface {
        draw_text_to_size(
            &self.layout,
            &self.draw_config.text_color,
            text,
            self.draw_config.font_pixel_height,
        )
    }
    pub fn draw_marker(&self, menut_type: &MenuType) -> Option<ImageSurface> {
        match menut_type {
            MenuType::Radio(state) => {
                let size = self.draw_config.marker_size;
                let color = self
                    .draw_config
                    .marker_color
                    .unwrap_or(self.draw_config.text_color);

                let surf = new_surface((size, size));
                let ctx = Context::new(&surf).unwrap();
                ctx.set_source_color(&color);

                let center = size as f64 / 2.;
                let line_width = (size as f64 / 10.).ceil();
                let radius = center - line_width / 2.;
                ctx.set_line_width(line_width);
                ctx.arc(center, center, radius, Z, 2. * PI);
                ctx.stroke().unwrap();

                if *state {
                    let radius = size as f64 / 5.;
                    ctx.arc(center, center, radius, Z, 2. * PI);
                    ctx.fill().unwrap();
                }

                Some(surf)
            }
            MenuType::Check(state) => {
                let size = self.draw_config.marker_size;
                let color = self
                    .draw_config
                    .marker_color
                    .unwrap_or(self.draw_config.text_color);

                let surf = new_surface((size, size));
                let ctx = Context::new(&surf).unwrap();
                ctx.set_source_color(&color);

                ctx.rectangle(Z, Z, size as f64, size as f64);
                ctx.set_line_width((size as f64 / 5.).ceil());
                ctx.stroke().unwrap();

                if *state {
                    let inner_size = (size as f64 * 0.5).ceil();
                    let start = (size as f64 - inner_size) / 2.;
                    ctx.rectangle(start, start, inner_size, inner_size);
                    ctx.fill().unwrap();
                }

                Some(surf)
            }
            MenuType::Parent(vec) => todo!(),
            _ => None,
        }
    }
}
