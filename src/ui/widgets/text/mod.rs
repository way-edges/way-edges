use std::cell::Cell;
use std::{rc::Rc, time::Duration};

use cairo::ImageSurface;
use chrono::{Local, Utc};
use educe::Educe;
use gtk::gdk::RGBA;
use gtk::glib;
use gtk::pango::Layout;
use interval_task::runner::Runner;

use crate::config::widgets::wrapbox::text::{TextConfig, TextPreset, TextUpdateTask};
use crate::ui::draws::util::{draw_text_to_size, ImageData};

use super::wrapbox::display::grid::DisplayWidget;
use super::wrapbox::expose::{BoxExpose, BoxRedrawFunc};

#[derive(Educe)]
#[educe(Debug)]
pub struct TextDrawer {
    pub fg_color: RGBA,
    pub layout: Layout,
    pub font_pixel_size: i32,
}
impl TextDrawer {
    pub fn new(config: &TextConfig) -> Self {
        let fg_color = config.fg_color;

        let layout = {
            let font_family = config.font_family.clone();
            let font_size = config.font_size << 10;

            let pc = pangocairo::pango::Context::new();
            let fm = pangocairo::FontMap::default();
            pc.set_font_map(Some(&fm));
            let mut desc = pc.font_description().unwrap();

            desc.set_size(font_size);
            if let Some(font_family) = font_family {
                desc.set_family(font_family.as_str());
            }
            pc.set_font_description(Some(&desc));
            pangocairo::pango::Layout::new(&pc)
        };

        Self {
            fg_color,
            layout,
            font_pixel_size: config.font_size,
        }
    }
    fn draw_text(&self, text: String) -> ImageData {
        draw_text_to_size(
            &self.layout,
            &self.fg_color,
            text.as_str(),
            self.font_pixel_size,
        )
        .try_into()
        .unwrap()
    }
}
impl Drop for TextDrawer {
    fn drop(&mut self) {
        log::info!("drop text drawer");
    }
}

struct TextEvents {
    queue_draw: BoxRedrawFunc,
}

fn match_preset(preset: TextPreset) -> (u64, TextUpdateTask) {
    match preset {
        TextPreset::Time { format, time_zone } => (
            1000,
            Box::new(move || {
                let a = if let Some(time_zone) = &time_zone {
                    use chrono::TimeZone;
                    let dt = Utc::now();
                    let tz: chrono_tz::Tz = time_zone
                        .parse()
                        .map_err(|e: chrono_tz::ParseError| e.to_string())?;
                    tz.from_utc_datetime(&dt.naive_utc()).naive_local()
                } else {
                    Local::now().naive_local()
                };
                Ok(a.format(format.as_str()).to_string())
            }),
        ),
        TextPreset::Custom {
            update_with_interval_ms,
        } => {
            if let Some((interval, f)) = update_with_interval_ms {
                (interval, f)
            } else {
                (999999, Box::new(|| Ok("no text present".to_string())))
            }
        }
    }
}

pub struct TextCtx {
    cache_content: Rc<Cell<ImageSurface>>,
    runner: Option<Runner<TextDrawer>>,
}
impl TextCtx {
    fn new(events: TextEvents, mut config: TextConfig) -> Result<Self, String> {
        // just use separate threads to run rather than one async thread.
        // incase some task takes too many time.
        let (runner, cache_content) = {
            let (interval, mut f) = match_preset(config.preset.take().unwrap());

            let (s, r) = async_channel::bounded(1);
            let mut runner = interval_task::runner::new_runner(
                Duration::from_millis(interval),
                move || TextDrawer::new(&config),
                move |inner| {
                    if let Ok(text) = f() {
                        s.force_send(inner.draw_text(text)).ok();
                    }
                    false
                },
            );
            runner.start().unwrap();

            let cache_content = Rc::new(Cell::new(
                r.recv_blocking()
                    .map_err(|_| "first frame fail to create".to_string())?
                    .into(),
            ));

            let queue_draw = events.queue_draw;
            // it's a while loop inside async block, so no matter weak or strong
            glib::spawn_future_local(glib::clone!(
                #[strong]
                cache_content,
                async move {
                    while let Ok(text_img_data) = r.recv().await {
                        cache_content.set(text_img_data.into());
                        queue_draw()
                    }
                    log::debug!("text update runner closed");
                }
            ));

            (Some(runner), cache_content)
        };

        Ok(Self {
            runner,
            cache_content,
        })
    }
}

impl DisplayWidget for TextCtx {
    fn get_size(&self) -> (f64, f64) {
        let c = &unsafe { self.cache_content.as_ptr().as_ref().unwrap() };
        (c.width() as f64, c.height() as f64)
    }
    fn content(&self) -> ImageSurface {
        unsafe { self.cache_content.as_ptr().as_ref().unwrap().clone() }
    }
}
impl Drop for TextCtx {
    fn drop(&mut self) {
        log::info!("drop text ctx");
        if let Some(r) = self.runner.take() {
            r.close().unwrap();
        }
    }
}

pub fn init_text(expose: &BoxExpose, config: TextConfig) -> Result<TextCtx, String> {
    let re = TextEvents {
        queue_draw: expose.update_func(),
    };
    TextCtx::new(re, config)
}
