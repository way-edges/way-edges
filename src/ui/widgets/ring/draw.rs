use cairo::{Format, ImageSurface};
use educe::Educe;
use gtk::pango::Layout;
use gtk::{gdk::RGBA, prelude::GdkCairoContextExt};

use crate::config::widgets::wrapbox::common::{Template, TemplateArg};
use crate::config::widgets::wrapbox::ring::RingConfig;
use crate::ui::draws::util::ImageData;
use crate::ui::draws::{shape::draw_fan, util::Z};
use crate::ui::draws::{transition_state, util};

pub struct RingCache {
    pub ring: ImageData,
    pub prefix: Option<ImageData>,
    pub suffix: Option<ImageData>,
}
unsafe impl Send for RingCache {}

impl RingCache {
    pub fn merge(&self, prefix_hide: bool, suffix_hide: bool, transition_y: f64) -> ImageSurface {
        let mut size = (self.ring.width, self.ring.height);

        let mut v = [None, None];

        if let Some(img_data) = &self.prefix {
            let img: ImageSurface = unsafe { img_data.temp_surface() };
            if prefix_hide {
                let visible_text_width = transition_state::calculate_transition(
                    transition_y,
                    (0., img_data.width as f64),
                )
                .ceil() as i32;
                v[0] = Some((img, visible_text_width, img_data.height));
                size.0 += visible_text_width;
                size.1 = size.1.max(img_data.height);
            } else {
                v[0] = Some((img, img_data.width, img_data.height));
                size.0 += img_data.width;
                size.1 = size.1.max(img_data.height);
            }
        }

        if let Some(img_data) = &self.suffix {
            let img: ImageSurface = unsafe { img_data.temp_surface() };
            if suffix_hide {
                let visible_text_width = transition_state::calculate_transition(
                    transition_y,
                    (0., img_data.width as f64),
                )
                .ceil() as i32;
                v[1] = Some((img, visible_text_width, img_data.height));
                size.0 += visible_text_width;
                size.1 = size.1.max(img_data.height);
            } else {
                v[1] = Some((img, img_data.width, img_data.height));
                size.0 += img_data.width;
                size.1 = size.1.max(img_data.height);
            }
        }

        let surf = util::new_surface(size);
        let ctx = cairo::Context::new(&surf).unwrap();

        if let Some((img, width, height)) = &v[0] {
            let h = ((size.1 as f64 - *height as f64) / 2.).floor();
            ctx.set_source_surface(img, Z, h).unwrap();
            ctx.rectangle(Z, h, *width as f64, *height as f64);
            ctx.fill().unwrap();

            ctx.translate(*width as f64, Z);
        }

        let h = (size.1 as f64 - self.ring.height as f64) / 2.;
        ctx.set_source_surface(unsafe { self.ring.temp_surface() }, Z, h)
            .unwrap();
        ctx.paint().unwrap();

        ctx.translate(self.ring.width as f64, Z);

        if let Some((img, _, height)) = &v[1] {
            let h = (size.1 as f64 - *height as f64) / 2.;
            ctx.set_source_surface(img, Z, h.floor()).unwrap();
            ctx.paint().unwrap();
        }

        surf
    }
}

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

    pub prefix: Option<Template>,
    pub suffix: Option<Template>,
}
impl Ring {
    pub fn new(config: &mut RingConfig) -> Self {
        let radius = config.common.radius;
        let ring_width = config.common.ring_width;
        let bg_color = config.common.bg_color;
        let fg_color = config.common.fg_color;
        let font_family = config.common.font_family.clone();
        let font_size = config.common.font_size;
        let (layout, bg_arc, inner_radius) =
            Self::initialize(radius, ring_width, &bg_color, font_family, font_size);

        let prefix = config.common.prefix.take();
        let suffix = config.common.suffix.take();

        Self {
            radius,
            fg_color,
            bg_arc,
            inner_radius,
            layout,
            prefix,
            suffix,
        }
    }
    fn initialize(
        radius: f64,
        ring_width: f64,
        bg_color: &RGBA,
        font_family: Option<String>,
        font_size: Option<f64>,
    ) -> (Layout, ImageSurface, f64) {
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

        // layout
        let pc = pangocairo::pango::Context::new();
        let fm = pangocairo::FontMap::default();
        pc.set_font_map(Some(&fm));
        let mut desc = pc.font_description().unwrap();
        desc.set_absolute_size(font_size.unwrap() * 1024.);
        if let Some(font_family) = font_family {
            desc.set_family(font_family.as_str());
        }
        pc.set_font_description(Some(&desc));
        let layout = pangocairo::pango::Layout::new(&pc);

        (layout, bg_surf, small_radius)
    }

    pub fn draw_ring(&self, progress: f64, preset: Option<String>) -> RingCache {
        let ring = {
            let radius = self.radius;
            let surf = util::new_surface((self.bg_arc.width(), self.bg_arc.height()));
            let ctx = cairo::Context::new(&surf).unwrap();

            ctx.set_source_surface(&self.bg_arc, Z, Z).unwrap();
            ctx.paint().unwrap();

            ctx.set_source_color(&self.fg_color);
            draw_fan(&ctx, (radius, radius), radius, -0.5, progress * 2. - 0.5);
            ctx.fill().unwrap();

            ctx.set_operator(cairo::Operator::Clear);
            draw_fan(&ctx, (radius, radius), self.inner_radius, 0., 2.);
            ctx.fill().unwrap();

            surf
        };

        let a = TemplateArg {
            float: Some(progress),
            preset: preset.as_deref(),
        };
        let prefix = self.prefix.as_ref().map(|t| {
            let text = t.parse(a.clone());
            util::draw_text(&self.layout, &self.fg_color, text.as_str()).into()
        });
        let suffix = self.suffix.as_ref().map(|t| {
            let text = t.parse(a);
            util::draw_text(&self.layout, &self.fg_color, text.as_str()).into()
        });

        RingCache {
            ring: ring.into(),
            prefix,
            suffix,
        }
    }
}
impl Drop for Ring {
    fn drop(&mut self) {
        log::debug!("drop ring");
    }
}
