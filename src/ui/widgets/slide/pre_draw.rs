use std::{f64::consts::PI, str::FromStr};

use gtk::{
    cairo::{self, ImageSurface, LinearGradient, Path},
    gdk::RGBA,
    prelude::GdkCairoContextExt,
};

use crate::ui::draws::util::{from_angel, new_surface, Z};

pub struct SlidePredraw {
    pub bg: ImageSurface,
    pub fg: ImageSurface,
    pub path: Path,
    pub shade: ImageSurface,
    pub stroke: ImageSurface,
}

fn predraw_err_handle(e: cairo::Error) -> String {
    format!("Slide Predraw error: {e}")
}
fn draw_slide_path(
    obtuse_angle: f64,
    radius: f64,
    size: (f64, f64),
    map_size: (i32, i32),
) -> Result<Path, String> {
    let ctx = cairo::Context::new(&new_surface((map_size.0, map_size.1), predraw_err_handle)?)
        .map_err(predraw_err_handle)?;

    // calculate
    let acute_angel = 180. - obtuse_angle;
    let stop_width = radius / from_angel(obtuse_angle / 2.).tan();
    println!("acute angle: {} stop width: {}", acute_angel, stop_width);

    // draw
    {
        let full = size.0 / from_angel(acute_angel);
        let percentage = (full - stop_width) / full;
        let full_y = size.0 / from_angel(acute_angel).tan();
        let x = size.0 * percentage;
        let y = full_y * percentage;
        println!(
            "full: {} percentage: {} x: {} y: {}",
            full, percentage, x, y
        );
        ctx.move_to(Z, Z);
        ctx.rel_line_to(x, y);

        let center = (size.0 - radius, full_y + stop_width);
        println!(
            "center: {:?}, angle1: {}, angle2: {}",
            center,
            (180. - obtuse_angle) / 180.,
            2.
        );
        ctx.arc(
            center.0,
            center.1,
            radius,
            (360. - acute_angel) / 180. * PI,
            // 0.5 * PI,
            // 2. * PI,
            2. * PI,
        );

        ctx.rel_line_to(Z, size.1 - 2. * stop_width - 2. * full_y);

        let center = (center.0, size.1 - center.1);
        ctx.arc(center.0, center.1, radius, Z, acute_angel / 180. * PI);
        ctx.line_to(Z, size.1);
        ctx.close_path();
    };
    ctx.copy_path().map_err(predraw_err_handle)
}

pub fn draw(size: (f64, f64), map_size: (i32, i32)) -> Result<SlidePredraw, String> {
    // provide
    let obtuse_angle = 120.;
    let radius = 20.;
    let fg = RGBA::from_str("#FFB847").unwrap();
    let bg = RGBA::from_str("#808080").unwrap();
    let border_color = RGBA::from_str("#646464").unwrap();
    let new_surface = move || new_surface((map_size.0, map_size.1), predraw_err_handle);

    let path = draw_slide_path(obtuse_angle, radius, size, map_size)?;

    let bg_surf = {
        let surf = new_surface()?;
        let ctx = cairo::Context::new(&surf).map_err(predraw_err_handle)?;
        ctx.rectangle(Z, Z, size.0, size.1);
        ctx.set_source_color(&bg);
        ctx.fill().map_err(predraw_err_handle)?;
        surf
    };

    let fg_surf = {
        let surf = new_surface()?;
        let ctx = cairo::Context::new(&surf).map_err(predraw_err_handle)?;
        ctx.set_source_color(&fg);
        ctx.append_path(&path);
        ctx.fill().map_err(predraw_err_handle)?;
        surf
    };

    let mask = {
        let start_point = (0., size.1 / 2.);
        let end_point = (size.0, size.1 / 2.);

        let surf = new_surface()?;
        let ctx = cairo::Context::new(&surf).map_err(predraw_err_handle)?;
        let lg = LinearGradient::new(size.0, start_point.1, end_point.0, end_point.1);
        lg.add_color_stop_rgba(0., Z, Z, Z, 0.);
        lg.add_color_stop_rgba(0.4, Z, Z, Z, 0.);
        lg.add_color_stop_rgba(1., Z, Z, Z, 0.7);
        ctx.set_source(&lg).map_err(predraw_err_handle)?;
        ctx.append_path(&path);
        ctx.fill().map_err(predraw_err_handle)?;
        surf
    };

    let stroke = {
        let surf = new_surface()?;
        let ctx = cairo::Context::new(&surf).map_err(predraw_err_handle)?;
        ctx.append_path(&path);
        ctx.set_source_color(&border_color);
        ctx.set_line_width(3.);
        ctx.stroke().map_err(predraw_err_handle)?;
        surf
    };

    Ok(SlidePredraw {
        bg: bg_surf,
        fg: fg_surf,
        path,
        shade: mask,
        stroke,
    })
}
