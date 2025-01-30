use cairo::{Format, ImageSurface};

use config::widgets::wrapbox::ring::RingConfig;
use cosmic_text::Color;
use util::color::cairo_set_color;
use util::draw::{draw_fan, new_surface};
use util::template::arg::{TemplateArgFloatParser, TEMPLATE_ARG_FLOAT};
use util::template::base::Template;
use util::text::{draw_text, TextConfig};
use util::Z;

use crate::animation::{calculate_transition, ToggleAnimationRc};
use crate::widgets::wrapbox::BoxTemporaryCtx;

use super::preset::RunnerResult;

#[derive(Debug)]
pub struct RingDrawer {
    radius: i32,
    ring_width: i32,
    bg_color: Color,
    fg_color: Color,

    prefix: Option<Template>,
    prefix_hide: bool,
    suffix: Option<Template>,
    suffix_hide: bool,

    font_family: Option<String>,
    font_size: i32,

    pub animation: ToggleAnimationRc,
}

impl RingDrawer {
    fn draw_ring(&self, progress: f64) -> ImageSurface {
        let big_radius = self.radius as f64;
        let small_radius = big_radius - self.ring_width as f64;
        let size = self.radius * 2;

        let surf = ImageSurface::create(Format::ARgb32, size, size).unwrap();
        let ctx = cairo::Context::new(&surf).unwrap();
        cairo_set_color(&ctx, self.bg_color);
        draw_fan(&ctx, (big_radius, big_radius), big_radius, 0., 2.);
        ctx.fill().unwrap();

        cairo_set_color(&ctx, self.fg_color);
        draw_fan(
            &ctx,
            (big_radius, big_radius),
            big_radius,
            -0.5,
            progress * 2. - 0.5,
        );
        ctx.fill().unwrap();

        ctx.set_operator(cairo::Operator::Clear);
        draw_fan(&ctx, (big_radius, big_radius), small_radius, 0., 2.);
        ctx.fill().unwrap();

        surf
    }
    fn draw_text(
        &self,
        progress: f64,
        preset_text: &str,
    ) -> (Option<ImageSurface>, Option<ImageSurface>) {
        // let layout = self.make_layout();
        let text_conf = TextConfig::new(
            self.font_family.as_deref(),
            None,
            self.fg_color,
            self.font_size,
        );

        let template_func = |template: &Template| {
            let text = template.parse(|parser| {
                let text = match parser.name() {
                    TEMPLATE_ARG_FLOAT => {
                        let parser = parser.downcast_ref::<TemplateArgFloatParser>().unwrap();
                        parser.parse(progress)
                    }
                    util::template::arg::TEMPLATE_ARG_RING_PRESET => preset_text.to_string(),
                    _ => unreachable!(),
                };
                text
            });

            draw_text(&text, text_conf).to_image_surface()
        };

        let prefix = self.prefix.as_ref().map(template_func);
        let suffix = self.suffix.as_ref().map(template_func);

        (prefix, suffix)
    }

    pub fn merge(
        &self,
        ring: ImageSurface,
        prefix: Option<ImageSurface>,
        suffix: Option<ImageSurface>,
    ) -> ImageSurface {
        let y = self.animation.borrow_mut().progress();

        let mut size = (self.radius * 2, self.radius * 2);

        let mut v = [None, None];

        if let Some(img) = prefix {
            let img_size = (img.width(), img.height());
            if self.prefix_hide {
                let visible_text_width =
                    calculate_transition(y, (0., img_size.0 as f64)).ceil() as i32;
                v[0] = Some((img, visible_text_width, img_size.1));
                size.0 += visible_text_width;
                size.1 = size.1.max(img_size.1);
            } else {
                v[0] = Some((img, img_size.0, img_size.1));
                size.0 += img_size.0;
                size.1 = size.1.max(img_size.1);
            }
        }

        if let Some(img) = suffix {
            let img_size = (img.width(), img.height());
            if self.suffix_hide {
                let visible_text_width =
                    calculate_transition(y, (0., img_size.0 as f64)).ceil() as i32;
                v[1] = Some((img, visible_text_width, img_size.1));
                size.0 += visible_text_width;
                size.1 = size.1.max(img_size.1);
            } else {
                v[1] = Some((img, img_size.0, img_size.1));
                size.0 += img_size.1;
                size.1 = size.1.max(img_size.1);
            }
        }

        let surf = new_surface(size);
        let ctx = cairo::Context::new(&surf).unwrap();

        if let Some((img, width, height)) = &v[0] {
            let h = ((size.1 as f64 - *height as f64) / 2.).floor();
            ctx.set_source_surface(img, Z, h).unwrap();
            ctx.rectangle(Z, h, *width as f64, *height as f64);
            ctx.fill().unwrap();

            ctx.translate(*width as f64, Z);
        }

        let h = (size.1 - self.radius * 2) as f64 / 2.;
        ctx.set_source_surface(ring, Z, h).unwrap();
        ctx.paint().unwrap();

        ctx.translate((self.radius * 2) as f64, Z);

        if let Some((img, _, height)) = &v[1] {
            let h = (size.1 as f64 - *height as f64) / 2.;
            ctx.set_source_surface(img, Z, h.floor()).unwrap();
            ctx.paint().unwrap();
        }

        surf
    }

    pub fn draw(&self, data: &RunnerResult) -> ImageSurface {
        let ring = self.draw_ring(data.progress);
        let (prefix, suffix) = self.draw_text(data.progress, &data.preset_text);
        self.merge(ring, prefix, suffix)
    }

    pub fn new(box_temp_ctx: &mut BoxTemporaryCtx, config: &mut RingConfig) -> Self {
        let radius = config.radius;
        let ring_width = config.ring_width;
        let bg_color = config.bg_color;
        let fg_color = config.fg_color;
        let font_family = config.font_family.clone();
        let font_size = config.font_size;

        let prefix = config.prefix.take();
        let suffix = config.suffix.take();

        let prefix_hide = config.prefix_hide;
        let suffix_hide = config.suffix_hide;

        let animation = box_temp_ctx.new_animation(config.text_transition_ms);

        Self {
            radius,
            fg_color,
            font_size,
            prefix,
            suffix,
            ring_width,
            bg_color,
            prefix_hide,
            suffix_hide,
            font_family,
            animation,
        }
    }
}
