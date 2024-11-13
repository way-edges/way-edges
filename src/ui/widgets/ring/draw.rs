use cairo::{Format, ImageSurface};
use educe::Educe;
use gtk::pango::Layout;
use gtk::{gdk::RGBA, prelude::GdkCairoContextExt};

use crate::config::widgets::wrapbox::ring::RingConfig;
use crate::ui::draws::util::{draw_text, horizon_center_combine, new_surface, ImageData};
use crate::ui::draws::{shape::draw_fan, util::Z};

pub struct ProgressCache {
    pub prefix_ring: ImageData,
    pub text: Option<ImageData>,
}
unsafe impl Send for ProgressCache {}

#[derive(Educe)]
#[educe(Debug)]
pub struct Ring {
    // config
    pub radius: f64,
    pub fg_color: RGBA,

    // from base
    pub bg_arc: ImageSurface,
    pub inner_radius: f64,
    pub layout: Layout,
    pub prefix_text: Option<ImageSurface>,
}
impl Ring {
    pub fn new(config: &RingConfig) -> Self {
        let radius = config.common.radius;
        let ring_width = config.common.ring_width;
        let bg_color = config.common.bg_color;
        let fg_color = config.common.fg_color;
        let prefix = config.common.prefix.clone();
        let font_family = config.common.font_family.clone();
        let font_size = config.common.font_size;
        let (layout, prefix_text, bg_arc, inner_radius) = Self::draw_base(
            radius,
            ring_width,
            &bg_color,
            &fg_color,
            prefix,
            font_family,
            font_size,
        );

        Self {
            radius,
            fg_color,
            bg_arc,
            inner_radius,
            layout,
            prefix_text,
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn draw_progress(&self, progress: f64, text: Option<String>) -> ProgressCache {
        let ring_surf = {
            let radius = self.radius;
            let surf = new_surface((self.bg_arc.width(), self.bg_arc.height()));
            let ctx = cairo::Context::new(&surf).unwrap();

            ctx.set_source_surface(&self.bg_arc, Z, Z).unwrap();
            ctx.paint().unwrap();

            ctx.set_source_color(&self.fg_color);
            draw_fan(&ctx, (radius, radius), radius, -0.5, progress * 2. - 0.5);
            ctx.fill().unwrap();

            ctx.set_operator(cairo::Operator::Clear);
            draw_fan(&ctx, (radius, radius), self.inner_radius, 0., 2.);
            ctx.fill().unwrap();

            // combine
            if let Some(pre_text) = self.prefix_text.as_ref() {
                horizon_center_combine(pre_text, &surf)
            } else {
                surf
            }
        };

        let text_surf =
            text.map(|text| draw_text(&self.layout, &self.fg_color, text.as_str()).into());

        ProgressCache {
            prefix_ring: ring_surf.into(),
            text: text_surf,
        }
    }
    pub fn draw_base(
        radius: f64,
        ring_width: f64,
        bg_color: &RGBA,
        fg_color: &RGBA,
        prefix: Option<String>,
        font_family: Option<String>,
        font_size: Option<f64>,
    ) -> (Layout, Option<ImageSurface>, ImageSurface, f64) {
        let big_radius = radius;
        let small_radius = big_radius - ring_width;
        let b_wh = (big_radius * 2.).ceil() as i32;

        let bg_surf = {
            let surf = ImageSurface::create(Format::ARgb32, b_wh, b_wh).unwrap();
            let ctx = cairo::Context::new(&surf).unwrap();

            ctx.set_source_color(bg_color);
            draw_fan(&ctx, (big_radius, big_radius), big_radius, 0., 2.);
            ctx.fill().unwrap();
            surf
        };

        let (ly, prefix_img) = {
            let pc = pangocairo::pango::Context::new();
            let fm = pangocairo::FontMap::default();
            pc.set_font_map(Some(&fm));
            let mut desc = pc.font_description().unwrap();
            desc.set_absolute_size(font_size.unwrap() * 1024.);
            if let Some(font_family) = font_family {
                desc.set_family(font_family.as_str());
            }
            pc.set_font_description(Some(&desc));
            let pl = pangocairo::pango::Layout::new(&pc);

            if let Some(prefix) = prefix {
                pl.set_text(prefix.as_str());
                let size = pl.pixel_size();

                let surf = ImageSurface::create(Format::ARgb32, size.0, size.1).unwrap();
                let ctx = cairo::Context::new(&surf).unwrap();
                ctx.set_antialias(cairo::Antialias::None);

                ctx.set_source_color(fg_color);
                pangocairo::functions::show_layout(&ctx, &pl);
                (pl, Some(surf))
            } else {
                (pl, None)
            }
        };

        (ly, prefix_img, bg_surf, small_radius)
    }
}
impl Drop for Ring {
    fn drop(&mut self) {
        log::debug!("drop ring");
    }
}
