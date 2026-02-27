use cairo::ImageSurface;

use config::def::{
    shared::{NumMargins, NumOrRelative},
    widgets::wrapbox::OutlookBoardConfig,
};
use cosmic_text::Color;
use smithay_client_toolkit::shell::wlr_layer::Anchor;
use util::{
    color::{cairo_set_color, color_mix},
    draw::{draw_rect_path, new_surface},
};

use super::OutlookDraw;

#[derive(Debug)]
pub struct DrawConf {
    margins: NumMargins,
    color: Color,
    border_radius: i32,

    corners: [bool; 4],
}
impl DrawConf {
    pub fn new(outlook: &OutlookBoardConfig, edge: Anchor, offset: NumOrRelative) -> Self {
        let corners = match offset.is_zero() {
            false => [true; 4],
            true => match edge {
                Anchor::LEFT => [false, true, true, false],
                Anchor::RIGHT => [true, false, false, true],
                Anchor::TOP => [false, false, true, true],
                Anchor::BOTTOM => [true, true, false, false],
                _ => unreachable!(),
            },
        };

        Self {
            margins: outlook.margins.clone(),
            color: outlook.color,
            border_radius: outlook.border_radius,
            corners,
        }
    }
}

impl OutlookDraw for DrawConf {
    fn translate_mouse_position(&self, pos: (f64, f64)) -> (f64, f64) {
        // pos - border - margin
        (
            pos.0 - self.margins.left as f64,
            pos.1 - self.margins.top as f64,
        )
    }
    fn draw(&mut self, content: ImageSurface) -> ImageSurface {
        let content_size = (content.width(), content.height());

        let border_radius = self.border_radius as f64;
        let corners = self.corners;

        // calculate_info for later use
        let total_size = (
            self.margins.left + self.margins.right + content_size.0,
            self.margins.top + self.margins.bottom + content_size.1,
        );

        // mix color of border color and shadow(black)
        let box_color = color_mix(Color::rgba(0, 0, 0, 0x22), self.color);

        // bg
        let path = draw_rect_path(
            border_radius,
            (total_size.0 as f64, total_size.1 as f64),
            corners,
        )
        .unwrap();
        let surf = new_surface(total_size);
        let ctx = cairo::Context::new(&surf).unwrap();
        cairo_set_color(&ctx, box_color);
        ctx.append_path(&path);
        ctx.fill().unwrap();

        ctx.set_source_surface(content, self.margins.left as f64, self.margins.top as f64)
            .unwrap();
        ctx.append_path(&path);
        ctx.fill().unwrap();

        surf
    }
}
