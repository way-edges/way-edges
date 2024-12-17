use std::f64::consts::PI;

use cairo::{Context, Format, Path};

use crate::ui::draws::util::Z;

/// do not use `PI`
pub fn draw_fan(ctx: &Context, point: (f64, f64), radius: f64, start: f64, end: f64) {
    ctx.arc(point.0, point.1, radius, start * PI, end * PI);
    ctx.line_to(point.0, point.1);
    ctx.close_path();
}

/// do not use `PI`
pub fn draw_fan_no_close(ctx: &Context, point: (f64, f64), radius: f64, start: f64, end: f64) {
    ctx.arc(point.0, point.1, radius, start * PI, end * PI);
    // ctx.line_to(point.0, point.1);
    // ctx.close_path();
}

pub fn draw_rect_path(radius: f64, size: (f64, f64), corners: [bool; 4]) -> Result<Path, String> {
    let surf =
        cairo::ImageSurface::create(Format::ARgb32, size.0.ceil() as i32, size.1.ceil() as i32)
            .unwrap();
    let ctx = cairo::Context::new(&surf).unwrap();

    // draw
    {
        // top left corner
        {
            ctx.move_to(Z, radius);
            if corners[0] {
                let center = (radius, radius);
                ctx.arc(center.0, center.1, radius, PI, 1.5 * PI);
            } else {
                ctx.line_to(Z, Z);
            }
            let x = size.0 - radius;
            let y = Z;
            ctx.line_to(x, y);
        }

        // top right corner
        {
            if corners[1] {
                let center = (size.0 - radius, radius);
                ctx.arc(center.0, center.1, radius, 1.5 * PI, 2. * PI);
            } else {
                ctx.line_to(size.0, Z);
            }
            let x = size.0;
            let y = size.1 - radius;
            ctx.line_to(x, y);
        }

        // bottom right corner
        {
            if corners[2] {
                let center = (size.0 - radius, size.1 - radius);
                ctx.arc(center.0, center.1, radius, 0., 0.5 * PI);
            } else {
                ctx.line_to(size.0, size.1);
            }
            let x = radius;
            let y = size.1;
            ctx.line_to(x, y);
        }

        // bottom left corner
        {
            if corners[3] {
                let center = (radius, size.1 - radius);
                ctx.arc(center.0, center.1, radius, 0.5 * PI, PI);
            } else {
                ctx.line_to(Z, size.1);
            }
            let x = Z;
            let y = radius;
            ctx.line_to(x, y);
        }

        ctx.close_path();
        Ok(ctx.copy_path().unwrap())
    }
}
