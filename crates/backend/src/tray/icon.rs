use std::io::Cursor;

use cairo::ImageSurface;
use gdk::{
    gdk_pixbuf::{Colorspace, Pixbuf},
    prelude::GdkCairoContextExt,
};
use system_tray::item::IconPixmap;

use util::Z;

fn new_image_surface_from_buf(buf: Pixbuf) -> ImageSurface {
    let width = buf.width();
    let height = buf.height();
    let format = cairo::Format::ARgb32;
    let surf = ImageSurface::create(format, width, height).unwrap();
    let context = cairo::Context::new(&surf).unwrap();

    context.set_source_pixbuf(&buf, Z, Z);
    context.paint().unwrap();

    surf
}
fn scale_image_to_size(img: ImageSurface, size: i32) -> ImageSurface {
    let scale = size as f64 / img.height() as f64;
    let width = (img.width() as f64 * scale).ceil() as i32;
    let height = (img.height() as f64 * scale).ceil() as i32;

    let surf = ImageSurface::create(cairo::Format::ARgb32, width, height).unwrap();
    let context = cairo::Context::new(&surf).unwrap();

    context.scale(scale, scale);
    context.set_source_surface(&img, Z, Z).unwrap();
    context.paint().unwrap();

    surf
}
// TODO: ICON THEME
pub fn parse_icon_given_name(name: &str, size: i32) -> Option<ImageSurface> {
    let file_path = freedesktop_icons::lookup(name)
        .with_size(size as u16)
        .find()?;
    let pixbuf = Pixbuf::from_file(file_path).ok()?;

    Some(scale_image_to_size(
        new_image_surface_from_buf(pixbuf),
        size,
    ))
}
pub fn parse_icon_given_data(vec: Vec<u8>) -> Option<ImageSurface> {
    ImageSurface::create_from_png(&mut Cursor::new(vec)).ok()
}
pub fn parse_icon_given_pixmaps(vec: &[IconPixmap], size: i32) -> Option<ImageSurface> {
    if vec.is_empty() {
        parse_icon_given_name("image-missing", size)
    } else {
        // we can do endian convert, but it's too hard
        // https://stackoverflow.com/a/10588779/21873016

        let pixmap = vec.last().unwrap();
        let mut pixels = pixmap.pixels.clone();

        // from ironbar
        for i in (0..pixels.len()).step_by(4) {
            let alpha = pixels[i];
            pixels[i] = pixels[i + 1];
            pixels[i + 1] = pixels[i + 2];
            pixels[i + 2] = pixels[i + 3];
            pixels[i + 3] = alpha;
        }

        let pixbuf = Pixbuf::from_mut_slice(
            &mut pixels,
            Colorspace::Rgb,
            true,
            8,
            pixmap.width,
            pixmap.height,
            pixmap.width * 4,
        );

        Some(scale_image_to_size(
            new_image_surface_from_buf(pixbuf),
            size,
        ))
    }
}
