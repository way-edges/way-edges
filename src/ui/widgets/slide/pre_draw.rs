use std::f64::consts::PI;

use gtk::{
    cairo::{self, ImageSurface, Path},
    gdk::RGBA,
    prelude::GdkCairoContextExt,
};

use crate::ui::draws::util::{from_angel, new_surface, Z};

pub struct SlidePredraw {
    pub bg: ImageSurface,
    // pub fg: ImageSurface,
    pub path: Path,
    // pub shade: ImageSurface,
    pub stroke: ImageSurface,
    pub slope_position: f64,
}

fn draw_slide_path(
    obtuse_angle: f64,
    radius: f64,
    size: (f64, f64),
    map_size: (i32, i32),
) -> Result<(Path, f64), String> {
    let ctx = cairo::Context::new(new_surface((map_size.0, map_size.1))).unwrap();

    // calculate
    let acute_angel = 180. - obtuse_angle;
    let stop_width = radius / from_angel(obtuse_angle / 2.).tan();
    log::debug!("acute angle: {} stop width: {}", acute_angel, stop_width);

    // draw
    {
        let full = size.0 / from_angel(acute_angel);
        let percentage = (full - stop_width) / full;
        let full_y = size.0 / from_angel(acute_angel).tan();
        let x = size.0 * percentage;
        let y = full_y * percentage;
        log::debug!(
            "full: {} percentage: {} x: {} y: {}",
            full,
            percentage,
            x,
            y
        );
        ctx.move_to(Z, Z);
        ctx.rel_line_to(x, y);

        let center = (size.0 - radius, full_y + stop_width);
        log::debug!(
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
        Ok((ctx.copy_path().unwrap(), full_y + stop_width))
    }
}

pub fn draw(
    size: (f64, f64),
    map_size: (i32, i32),
    bg: RGBA,
    border_color: RGBA,
    obtuse_angle: f64,
    radius: f64,
) -> Result<SlidePredraw, String> {
    // provide
    // let obtuse_angle = 120.;
    // let radius = 20.;
    let new_surface = move || new_surface((map_size.0, map_size.1));

    let (path, slope_position) = draw_slide_path(obtuse_angle, radius, size, map_size)?;

    let bg_surf = {
        let surf = new_surface();
        let ctx = cairo::Context::new(&surf).unwrap();
        ctx.set_source_color(&bg);
        ctx.append_path(&path);
        ctx.fill().unwrap();
        surf
    };

    // let fg_surf = {
    //     let surf = new_surface()?;
    //     let ctx = cairo::Context::new(&surf).unwrap();
    //     ctx.set_source_color(&fg);
    //     ctx.append_path(&path);
    //     ctx.fill().unwrap();
    //     surf
    // };

    // let mask = {
    //     let start_point = (0., size.1 / 2.);
    //     let end_point = (size.0, size.1 / 2.);
    //
    //     let surf = new_surface();
    //     let ctx = cairo::Context::new(&surf).unwrap();
    //     let lg = LinearGradient::new(start_point.0, start_point.1, end_point.0, end_point.1);
    //     lg.add_color_stop_rgba(0., Z, Z, Z, 0.);
    //     lg.add_color_stop_rgba(0.4, Z, Z, Z, 0.);
    //     lg.add_color_stop_rgba(1., Z, Z, Z, 0.7);
    //     ctx.set_source(&lg).unwrap();
    //     ctx.append_path(&path);
    //     ctx.fill().unwrap();
    //     surf
    // };

    let stroke = {
        let surf = new_surface();
        let ctx = cairo::Context::new(&surf).unwrap();
        ctx.append_path(&path);
        ctx.set_source_color(&border_color);
        ctx.set_line_width(3.);
        ctx.stroke().unwrap();
        surf
    };

    Ok(SlidePredraw {
        bg: bg_surf,
        // fg: fg_surf,
        path,
        // shade: mask,
        stroke,
        slope_position,
    })
}
