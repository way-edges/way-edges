use std::sync::atomic::AtomicPtr;

use lazy_static::lazy_static;
use pangocairo::pango;
use pangocairo::pango::prelude::FontMapExt;

mod raw;

pub fn get_pango_context() -> pango::Context {
    lazy_static! {
        static ref FONT_MAP: AtomicPtr<pango::FontMap> = {
            let map = pangocairo::FontMap::default();
            let p = "/tmp/way-edges/slide-font.otf";
            std::fs::write(p, raw::RAW_FONT).unwrap();
            map.add_font_file(p).unwrap();
            AtomicPtr::new(Box::into_raw(Box::new(map)))
        };
    }

    let ctx = unsafe { FONT_MAP.load(std::sync::atomic::Ordering::Relaxed).as_ref() }
        .unwrap()
        .create_context();
    let mut desc = ctx.font_description().unwrap();
    desc.set_family("WayEdges-Slide");
    ctx.set_font_description(Some(&desc));
    ctx
}
