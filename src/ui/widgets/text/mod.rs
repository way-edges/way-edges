/// NOTE: This widget can not be used directly
use std::cell::Cell;
use std::{rc::Rc, time::Duration};

use cairo::ImageSurface;
use educe::Educe;
use gtk::gdk::RGBA;
use gtk::glib;
use gtk::pango::Layout;
use interval_task::runner::Runner;

use crate::config::widgets::ring::RingConfig;
use crate::ui::draws::util::{draw_text, ImageData};

use super::wrapbox::display::grid::DisplayWidget;
use super::wrapbox::expose::BoxExposeRc;

#[derive(Educe)]
#[educe(Debug)]
pub struct TextDrawer {
    pub fg_color: RGBA,
    pub layout: Layout,
}
impl TextDrawer {
    pub fn new(config: &RingConfig) -> Self {
        let fg_color = config.common.fg_color;

        let layout = {
            let font_family = config.common.font_family.clone();
            let font_size = config.common.font_size;

            let pc = pangocairo::pango::Context::new();
            let fm = pangocairo::FontMap::default();
            pc.set_font_map(Some(&fm));
            let mut desc = pc.font_description().unwrap();
            desc.set_size(font_size);
            if let Some(font_family) = font_family {
                desc.set_family(font_family.as_str());
                pc.set_font_description(Some(&desc));
            }
            pangocairo::pango::Layout::new(&pc)
        };

        Self { fg_color, layout }
    }
    fn draw_text(&self, text: String) -> ImageData {
        draw_text(&self.layout, &self.fg_color, text.as_str()).into()
    }
}
impl Drop for TextDrawer {
    fn drop(&mut self) {
        log::debug!("drop text drawer");
    }
}

struct TextEvents {
    queue_draw: Box<dyn FnMut() + 'static>,
}

pub struct RingCtx {
    cache_content: Rc<Cell<ImageSurface>>,
    runner: Option<Runner<TextDrawer>>,
}
impl RingCtx {
    fn new(events: TextEvents, mut config: RingConfig) -> Result<Self, String> {
        // just use separate threads to run rather than one async thread.
        // incase some task takes too many time.
        let (runner, cache_content) = {
            let (interval, mut f) = config.update_with_interval_ms.take().unwrap();

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
                    .map_err(|_| "first surface fail to create".to_string())?
                    .into(),
            ));

            let mut queue_draw = events.queue_draw;
            // it's a while loop inside async block, so no matter weak or strong
            glib::spawn_future_local(glib::clone!(
                #[strong]
                cache_content,
                async move {
                    while let Ok(text_img_data) = r.recv().await {
                        cache_content.set(text_img_data.into());
                        queue_draw()
                    }
                    log::warn!("ring update runner closed");
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

impl DisplayWidget for RingCtx {
    fn get_size(&mut self) -> (f64, f64) {
        let c = &unsafe { self.cache_content.as_ptr().as_ref().unwrap() };
        (c.width() as f64, c.height() as f64)
    }
    fn content(&mut self) -> ImageSurface {
        unsafe { self.cache_content.as_ptr().as_ref().unwrap().clone() }
    }
}
impl Drop for RingCtx {
    fn drop(&mut self) {
        log::debug!("drop ring ctx");
        if let Some(r) = self.runner.take() {
            r.close().unwrap();
        }
    }
}

pub fn init_ring(expose: &BoxExposeRc, config: RingConfig) -> Result<RingCtx, String> {
    let re = {
        let expose = expose.borrow_mut();
        let s = expose.update_func();
        TextEvents {
            queue_draw: Box::new(s),
        }
    };
    RingCtx::new(re, config)
}
