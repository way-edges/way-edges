use cairo::freetype as ft;
use cairo::FontFace;
use std::rc::Rc;

mod font;

static mut FONTFACE: Option<Result<FontFace, String>> = None;

pub fn get_font_face() -> Result<FontFace, String> {
    unsafe {
        if FONTFACE.as_ref().is_none() {
            let fc = ft::Library::init()
                .map_err(|e| format!("Init freetype error: {e}"))
                .and_then(|lib| {
                    let fontface = lib
                        .new_memory_face(Rc::new(font::RAW_FONT.to_vec()), 0)
                        .map_err(|e| format!("Init ft fontface error: {e}"))?;
                    FontFace::create_from_ft(&fontface)
                        .map_err(|e| format!("Init cairo fontface error: {e}"))
                });
            FONTFACE = Some(fc);
        };
        FONTFACE.as_ref().unwrap().clone()
    }
}
