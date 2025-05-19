use cairo::ImageSurface;
use cosmic_text::{
    Attrs, Buffer, Color, Family, FontSystem, LayoutRunIter, Metrics, Shaping, SwashCache, Weight,
};

use crate::pre_multiply_and_to_little_endian_argb;

use super::slide_font::include_slide_font;

extern crate alloc;

static FONT_SYSTEM: std::sync::LazyLock<std::sync::Mutex<FontSystem>> =
    std::sync::LazyLock::new(|| {
        let mut f = FontSystem::new();
        f.db_mut().load_font_data(include_slide_font!().to_vec());
        std::sync::Mutex::new(f)
    });

static SWASH_CACHE: std::sync::LazyLock<std::sync::Mutex<SwashCache>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(SwashCache::new()));

pub struct Canvas {
    pub canvas_buffer: Box<[u8]>,
    pub height: i32,
    pub width: i32,
    pub stride: i32,
}

impl Canvas {
    fn new(width: i32, height: i32) -> Self {
        let canvas_buffer = vec![0; width as usize * height as usize * 4].into_boxed_slice();
        Self {
            canvas_buffer,
            height,
            width,
            stride: width * 4,
        }
    }
    fn set_pixel_color(&mut self, color: Color, x: i32, y: i32) {
        let stride = self.stride;

        let line = stride * y;
        let chunk_start = (x * 4) + line;
        if chunk_start as usize > self.canvas_buffer.len() - 4 {
            return;
        }

        let color = pre_multiply_and_to_little_endian_argb(color.as_rgba());
        self.canvas_buffer[chunk_start as usize..(chunk_start + 4) as usize]
            .copy_from_slice(&color);
    }

    pub fn to_image_surface(self) -> ImageSurface {
        let Self {
            canvas_buffer,
            height,
            width,
            stride,
        } = self;
        ImageSurface::create_for_data(canvas_buffer, cairo::Format::ARgb32, width, height, stride)
            .unwrap()
    }
}

fn measure_text_size(
    buffer: &Buffer,
    swash_cache: &mut SwashCache,
    font_system: &mut FontSystem,
) -> (i32, i32) {
    // Get the layout runs
    let layout_runs: LayoutRunIter = buffer.layout_runs();
    let mut run_width: f32 = 0.;
    let mut run_height_high: f32 = f32::MIN;
    let mut last_run = None;

    for run in layout_runs {
        run_width = run_width.max(run.line_w);
        run_height_high = run_height_high.max(run.line_y);
        last_run = Some(run);
    }

    if let Some(run) = last_run {
        let mut m = 0;
        for g in run.glyphs {
            let img = swash_cache
                .get_image(font_system, g.physical((0., 0.), 1.).cache_key)
                .as_ref()
                .unwrap();
            m = m.max(img.placement.height as i32 - img.placement.top);
        }

        run_height_high += m as f32;
    }

    (run_width.ceil() as i32, run_height_high.ceil() as i32)
}

#[derive(Debug, Clone, Copy)]
pub struct TextConfig<'a> {
    pub family: Family<'a>,
    pub weight: Option<Weight>,
    pub color: Color,
    pub size: i32,
}
impl<'a> TextConfig<'a> {
    pub fn new(family: Family<'a>, weight: Option<u16>, color: Color, size: i32) -> Self {
        Self {
            family,
            weight: weight.map(Weight),
            color,
            size,
        }
    }
}

fn draw_text_inner(
    text: &str,
    config: TextConfig,
    swash_cache: &mut SwashCache,
    font_system: &mut FontSystem,
) -> Canvas {
    let height = config.size as f32;
    let metrics = Metrics::new(height, height);
    let mut buffer = Buffer::new_empty(metrics);

    let mut attrs = Attrs::new();
    if let Some(weight) = config.weight {
        attrs = attrs.weight(weight);
    }
    attrs = attrs.family(config.family);

    buffer.set_text(font_system, text, &attrs, Shaping::Advanced);
    buffer.shape_until_scroll(font_system, true);

    let (width, height) = measure_text_size(&buffer, swash_cache, font_system);
    let mut canvas = Canvas::new(width, height);

    buffer.draw(
        font_system,
        swash_cache,
        config.color,
        |x, y, w, h, color| {
            if color.a() == 0 || x < 0 || x >= width || y < 0 || y >= height || w != 1 || h != 1 {
                return;
            }
            canvas.set_pixel_color(color, x, y)
        },
    );

    canvas
}

pub fn draw_text(text: &str, config: TextConfig) -> Canvas {
    let mut swash_cache = SWASH_CACHE.lock().unwrap();
    let mut font_system = FONT_SYSTEM.lock().unwrap();
    draw_text_inner(text, config, &mut swash_cache, &mut font_system)
}
