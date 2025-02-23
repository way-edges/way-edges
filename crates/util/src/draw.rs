use std::f64::consts::PI;

use cairo::{Format, ImageSurface, Path};

use crate::Z;

pub fn new_surface(size: (i32, i32)) -> ImageSurface {
    ImageSurface::create(Format::ARgb32, size.0, size.1).unwrap()
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

pub fn draw_fan(ctx: &cairo::Context, point: (f64, f64), radius: f64, start: f64, end: f64) {
    ctx.arc(point.0, point.1, radius, start * PI, end * PI);
    ctx.line_to(point.0, point.1);
    ctx.close_path();
}

#[allow(clippy::too_many_arguments)]
pub fn copy_pixmap(
    src_data: &[u8],
    src_width: usize,
    src_height: usize,
    dst_data: &mut [u8],
    dst_width: usize,
    dst_height: usize,
    x: isize,
    y: isize,
) {
    let (sx_start, dx_start, copy_width) = {
        let sx_start = (-x).max(0) as usize;
        let dx_start = x.max(0) as usize;

        let remaining_width_src = src_width.saturating_sub(sx_start) as isize;
        let remaining_width_dst = dst_width.saturating_sub(dx_start) as isize;

        let copy_width = remaining_width_src.min(remaining_width_dst).max(0) as usize;
        (sx_start, dx_start, copy_width)
    };

    let (sy_start, dy_start, copy_height) = {
        let sy_start = (-y).max(0) as usize;
        let dy_start = y.max(0) as usize;

        let remaining_height_src = src_height.saturating_sub(sy_start) as isize;
        let remaining_height_dst = dst_height.saturating_sub(dy_start) as isize;

        let copy_height = remaining_height_src.min(remaining_height_dst).max(0) as usize;
        (sy_start, dy_start, copy_height)
    };

    if copy_width == 0 || copy_height == 0 {
        return;
    }

    for row in 0..copy_height {
        let src_row = sy_start + row;
        let dst_row = dy_start + row;

        let src_start = (src_row * src_width + sx_start) * 4;
        let src_end = src_start + copy_width * 4;
        let dst_start = (dst_row * dst_width + dx_start) * 4;
        let dst_end = dst_start + copy_width * 4;

        dst_data[dst_start..dst_end].copy_from_slice(&src_data[src_start..src_end]);
    }
}
