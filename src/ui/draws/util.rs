use gtk::cairo::{self, Context, Format, ImageSurface};

pub fn copy_surface(src: &ImageSurface) -> ImageSurface {
    let dst = ImageSurface::create(Format::ARgb32, src.width(), src.height()).unwrap();
    let ctx = cairo::Context::new(&dst).unwrap();
    copy_surface_to_context(&ctx, src);
    dst
}

pub fn copy_surface_to_context(dst: &Context, src: &ImageSurface) {
    dst.set_source_surface(src, 0., 0.).unwrap();
    dst.rectangle(0., 0., src.width().into(), src.height().into());
    dst.fill().unwrap();
}
