mod custom;
mod freedesktop;

use std::{io::Cursor, path::PathBuf};

use cairo::ImageSurface;
use resvg::{tiny_skia, usvg};
use system_tray::item::IconPixmap;

use util::{pre_multiply_and_to_little_endian_argb, Z};

pub fn parse_icon_given_data(vec: &Vec<u8>, size: i32) -> Option<ImageSurface> {
    ImageSurface::create_from_png(&mut Cursor::new(vec))
        .ok()
        .map(|img| scale_image_to_size(img, size))
}

pub fn parse_icon_given_pixmaps(vec: &[IconPixmap], size: i32) -> Option<ImageSurface> {
    if vec.is_empty() {
        None
    } else {
        let pixmap = vec.last().unwrap();

        // ARGB
        let mut pixels = pixmap.pixels.clone();

        // pre multiply
        for i in (0..pixels.len()).step_by(4) {
            // little endian (BGRA)
            let res = pre_multiply_and_to_little_endian_argb([
                pixels[i + 1],
                pixels[i + 2],
                pixels[i + 3],
                pixels[i],
            ]);

            pixels[i] = res[0];
            pixels[i + 1] = res[1];
            pixels[i + 2] = res[2];
            pixels[i + 3] = res[3];
        }

        let img = ImageSurface::create_for_data(
            pixels,
            cairo::Format::ARgb32,
            pixmap.width,
            pixmap.height,
            pixmap.width * 4,
        )
        .unwrap();

        Some(scale_image_to_size(img, size))
    }
}

pub enum IconThemeNameOrPath<'a> {
    Name(Option<&'a str>),
    Path(&'a str),
}

pub fn parse_icon_given_name(
    name: &str,
    size: i32,
    theme: IconThemeNameOrPath,
) -> Option<ImageSurface> {
    let f = match theme {
        IconThemeNameOrPath::Name(n) => freedesktop::find_icon(name, size, n),
        IconThemeNameOrPath::Path(p) => custom::find_icon(p, name),
    }?;

    draw_icon_file(f, size)
}

pub fn fallback_icon(size: i32, theme: Option<&str>) -> Option<ImageSurface> {
    let f = freedesktop::fallback_icon(size, theme)?;
    draw_icon_file(f, size)
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

fn draw_icon_file(file_path: PathBuf, size: i32) -> Option<ImageSurface> {
    let ext = file_path.extension().unwrap().to_str().unwrap();
    let img = match ext {
        "png" => load_png(&file_path),
        "svg" => load_svg(&file_path),
        _ => {
            log::error!("draw_icon_file error: unsupported file extension: {ext}");
            return None;
        }
    }?;

    Some(scale_image_to_size(img, size))
}

fn load_png(p: &PathBuf) -> Option<ImageSurface> {
    let contents = std::fs::read(p)
        .inspect_err(|e| log::error!("load_png error: {e}"))
        .ok()?;
    ImageSurface::create_from_png(&mut Cursor::new(contents))
        .inspect_err(|f| log::error!("load_png to surface error: {f}"))
        .ok()
}

fn load_svg(p: &PathBuf) -> Option<ImageSurface> {
    let tree = {
        let opt = usvg::Options {
            resources_dir: std::fs::canonicalize(p)
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf())),
            ..usvg::Options::default()
        };
        // NOTE: WE DO NOT EXPECT TEXT TO APPEAR INSIDE, SHOULD WE?
        // opt.fontdb.load_system_fonts();

        let svg_data = std::fs::read(p)
            .inspect_err(|f| log::error!("load_svg error: {f}"))
            .ok()?;
        usvg::Tree::from_data(&svg_data, &opt)
            .inspect_err(|f| log::error!("parse svg data error: {f}"))
            .ok()?
    };

    let pixmap_size = tree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

    // RGBA
    let mut pixels = pixmap.take();

    // TO BGRA(little endian of ARGB)
    for i in (0..pixels.len()).step_by(4) {
        let bgra = [pixels[i + 2], pixels[i + 1], pixels[i], pixels[i + 3]];
        pixels[i] = bgra[0];
        pixels[i + 1] = bgra[1];
        pixels[i + 2] = bgra[2];
        pixels[i + 3] = bgra[3];
    }

    ImageSurface::create_for_data(
        pixels,
        cairo::Format::ARgb32,
        pixmap_size.width() as i32,
        pixmap_size.height() as i32,
        pixmap_size.width() as i32 * 4,
    )
    .inspect_err(|f| log::error!("load_svg to surface error: {f}"))
    .ok()
}
