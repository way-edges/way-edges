use std::{collections::HashMap, f64::consts::PI, str::FromStr };

use cairo::{Context, Format, ImageSurface, Path};
use gtk::{gdk::RGBA, prelude::GdkCairoContextExt};

use crate::ui::draws::{draw_fan, util::Z};

fn draw_rect_path(radius: f64, size: (f64, f64), corners: [bool; 4]) -> Result<Path, String> {
    let surf =
        cairo::ImageSurface::create(Format::ARgb32, size.0.ceil() as i32, size.1.ceil() as i32)
            .unwrap();
    let ctx = cairo::Context::new(&surf).unwrap();

    // calculate
    let acute_angel = 90.;
    println!("acute angle: {}", acute_angel);

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

#[derive(Debug)]
struct BoxDraws {
    border_path: Path,
    border: ImageSurface,
    window_path: Path,
    window: ImageSurface,
    window_shadow: ImageSurface,
    content_size: (f64, f64),
    size: (f64, f64),
    margins: [f64; 4],

    content_box_size: (f64, f64),
    startoff_point: (f64, f64),
}

impl BoxDraws {
    /// margins: left, top, right, bottom
    fn new(
        content_size: (f64, f64),
        margins: Option<[f64; 4]>,
        border_color: RGBA,
        box_color: Option<RGBA>,
        radius_percentage: f64,
        size_factors: (f64, f64),
    ) -> Self {
        let ([content_box_size, size, startoff_point], margins) =
            Self::calculate_info(content_size, margins, size_factors);
        // let content_box_size = (
        //     (content_size.0 + margins[0] + margins[2]),
        //     (content_size.1 + margins[1] + margins[3]),
        // );
        // let size = ((content_box_size.0) / 0.75, (content_box_size.1) / 0.85);
        let box_color = box_color.unwrap_or_else(|| {
            let mut shade = RGBA::BLACK;
            shade.set_alpha(0.2);
            let one = shade;
            let two = border_color;
            let a = 1. - (1. - one.alpha()) * (1. - two.alpha());
            let r = (one.red() * one.alpha() + two.red() * two.alpha() * (1. - one.alpha())) / a;
            let g =
                (one.green() * one.alpha() + two.green() * two.alpha() * (1. - one.alpha())) / a;
            let b = (one.blue() * one.alpha() + two.blue() * two.alpha() * (1. - one.alpha())) / a;
            RGBA::new(r, g, b, a)
        });
        println!(
            "bg color: {:?}, border color: {:?}",
            box_color, border_color
        );

        let new_surface =
            move |s: (i32, i32)| ImageSurface::create(Format::ARgb32, s.0, s.1).unwrap();

        let (border_path, border) = {
            let path =
                draw_rect_path(size.0 * radius_percentage, size, [false, true, true, false])
                    .unwrap();
            let map_size = (size.0.ceil() as i32, size.1.ceil() as i32);
            let surf = new_surface(map_size);
            let ctx = cairo::Context::new(&surf).unwrap();
            ctx.set_source_color(&border_color);
            ctx.append_path(&path);
            ctx.fill().unwrap();
            (path, surf)
        };

        let (window_path, window, window_shadow) = {
            let map_size = (
                content_box_size.0.ceil() as i32,
                content_box_size.1.ceil() as i32,
            );
            let path = draw_rect_path(
                content_box_size.0 * radius_percentage,
                content_box_size,
                [true, true, true, true],
            )
            .unwrap();
            let bg_surf = {
                let surf = new_surface(map_size);
                let ctx = cairo::Context::new(&surf).unwrap();
                ctx.set_source_color(&box_color);
                ctx.append_path(&path);
                ctx.fill().unwrap();
                surf
            };
            let shadow_surf = {
                fn inside_grandient(p: [f64; 4], c: &RGBA) -> cairo::LinearGradient {
                    let t = cairo::LinearGradient::new(p[0], p[1], p[2], p[3]);
                    t.add_color_stop_rgba(
                        0.,
                        c.red().into(),
                        c.green().into(),
                        c.blue().into(),
                        0.4,
                    );
                    t.add_color_stop_rgba(
                        0.3,
                        c.red().into(),
                        c.green().into(),
                        c.blue().into(),
                        0.1,
                    );
                    t.add_color_stop_rgba(
                        1.,
                        c.red().into(),
                        c.green().into(),
                        c.blue().into(),
                        0.,
                    );
                    t
                }

                let surf = new_surface(map_size);
                let ctx = cairo::Context::new(&surf).unwrap();
                let g = |p: [f64; 4], c: &RGBA| {
                    let t = inside_grandient(p, c);
                    ctx.set_source(t).unwrap();
                    ctx.append_path(&path);
                    ctx.fill().unwrap();
                };

                g([Z, Z, content_box_size.0 * 0.3, Z], &RGBA::BLACK);
                g([Z, Z, Z, content_box_size.0 * 0.3], &RGBA::BLACK);
                g(
                    [
                        Z,
                        content_box_size.1,
                        Z,
                        content_box_size.1 - content_box_size.0 * 0.3,
                    ],
                    &RGBA::BLACK,
                );
                surf
            };

            (path, bg_surf, shadow_surf)
        };
        BoxDraws {
            border_path,
            border,
            window_path,
            window,
            window_shadow,
            startoff_point,
            content_size,
            content_box_size,
            size,
            margins,
        }
    }

    fn calculate_info(
        content_size: (f64, f64),
        margins: Option<[f64; 4]>,
        size_factors: (f64, f64),
    ) -> ([(f64, f64); 3], [f64; 4]) {
        let margins = margins.unwrap_or([0., 0., 0., 0.]);
        let content_box_size = (
            (content_size.0 + margins[0] + margins[2]),
            (content_size.1 + margins[1] + margins[3]),
        );
        let size = (
            (content_box_size.0) / size_factors.0,
            (content_box_size.1) / size_factors.1,
        );
        let startoff_point = (
            (size.0 - content_box_size.0) / 2.,
            (size.1 - content_box_size.1) / 2.,
        );
        ([content_box_size, size, startoff_point], margins)
    }

    /// get context_size
    fn calculate_info_reverse(
        map_size: (f64, f64),
        margins: Option<[f64; 4]>,
        size_factors: (f64, f64),
    ) -> (f64, f64) {
        let margins = margins.unwrap_or([0., 0., 0., 0.]);
        (
            (map_size.0 - margins[0] - margins[2]) * size_factors.0,
            (map_size.1 - margins[1] - margins[3]) * size_factors.1,
        )
    }

    fn with_box(&self, content: ImageSurface) -> ImageSurface {
        println!("self: {self:?}");
        let surf = ImageSurface::create(
            Format::ARgb32,
            self.size.0.ceil() as i32,
            self.size.1.ceil() as i32,
        )
        .unwrap();
        let ctx = cairo::Context::new(&surf).unwrap();

        // border
        ctx.set_source_surface(&self.border, Z, Z).unwrap();
        ctx.rectangle(Z, Z, self.size.0, self.size.1);
        ctx.fill().unwrap();

        ctx.translate(self.startoff_point.0, self.startoff_point.1);

        // content background
        ctx.set_source_surface(&self.window, Z, Z).unwrap();
        ctx.rectangle(Z, Z, self.content_box_size.0, self.content_box_size.1);
        ctx.fill().unwrap();

        // content
        ctx.set_source_surface(&content, self.margins[0], self.margins[1])
            .unwrap();
        ctx.append_path(&self.window_path);
        ctx.fill().unwrap();

        // shadow
        ctx.set_source_surface(&self.window_shadow, Z, Z).unwrap();
        ctx.rectangle(Z, Z, self.content_box_size.0, self.content_box_size.1);
        ctx.fill().unwrap();

        surf
    }
}


fn draw_ring(
    big_radius: f64,
    small_radius: f64,
    bg: &RGBA,
    fg: &RGBA,
    progress: f64,
) -> ImageSurface {
    let b_wh = (big_radius * 2.).ceil() as i32;

    let surf = ImageSurface::create(Format::ARgb32, b_wh, b_wh).unwrap();
    let ctx = cairo::Context::new(&surf).unwrap();

    ctx.set_source_color(bg);
    draw_fan(&ctx, (big_radius, big_radius), big_radius, 0., 2.);
    ctx.fill().unwrap();

    ctx.set_source_color(fg);
    draw_fan(
        &ctx,
        (big_radius, big_radius),
        big_radius,
        0.,
        progress * 2.,
    );
    ctx.fill().unwrap();

    ctx.set_operator(cairo::Operator::Clear);
    ctx.set_source_rgba(Z, Z, Z, Z);
    ctx.arc(big_radius, big_radius, small_radius, 0., 2. * PI);
    ctx.fill().unwrap();

    surf
}

trait BoxedWidget {
    fn get_size(&self) -> (f64, f64);
    fn content(&self) -> ImageSurface;
}

type BoxWidgetIndex = (usize, usize);

struct BoxWidgets {
    /// first row, second col
    /// [
    ///   [a, b ,c],
    ///   [d, e, f],
    /// ]
    ws: Vec<Option<Vec<Option<Box<dyn BoxedWidget>>>>>,
    row_col_num: (usize, usize),
    max_size: Option<[(f64, BoxWidgetIndex); 2]>,
    size_change_map: HashMap<BoxWidgetIndex, (f64, f64)>,
}
impl BoxWidgets {
    fn new() -> Self {
        Self {
            ws: vec![],
            size_change_map: HashMap::new(),
            max_size: None,
            row_col_num: (0, 0),
        }
    }
    fn add(
        &mut self,
        w: Box<dyn BoxedWidget + 'static>,
        position: (isize, isize),
    ) -> (usize, usize) {
        let pos: (usize, usize) = (0, 0);
        pos.0 = if position.0 == -1 {
            self.row_col_num.0
        } else if position.0 >= 0 {
            position.0 as usize
        } else {
            panic!("position must be positive or -1");
        };
        pos.1 = if position.1 == -1 {
            self.row_col_num.1
        } else if position.1 >= 0 {
            position.1 as usize
        } else {
            panic!("position must be positive or -1");
        };
        if self.ws.len()-1 < pos.0 {
            self.ws.
        }

        self.ws.push(w);
        self.size_change_map.insert(k, v);
    }
    fn total_size(&self) -> (f64, f64) {}
}

fn draw(ctx: &Context) {
    let map_size = (40., 200.);
    let size_factors = (0.75, 0.85);
    let margins = Some([3., 3., 3., 3.]);
    let bws = BoxWidgets::new();
    {
        let radius = 15.;
        let ring_width = 5.;
        let ring_bg = RGBA::from_str("#9F9F9F").unwrap();
        let ring_fg = RGBA::from_str("#F1FA8C").unwrap();
        let content = { draw_ring(radius, radius - ring_width, &ring_bg, &ring_fg, 0.3) };
    };
    // let content_size =
    let border_color = RGBA::from_str("#C18F4A").unwrap();
    let radius_percentage = 0.3;
    let b = BoxDraws::new(
        content_size,
        margins,
        border_color,
        // Some(RGBA::GREEN),
        None,
        radius_percentage,
        (0.75, 0.85),
    );

    ctx.set_source_surface(b.with_box(content), Z, Z).unwrap();
    ctx.rectangle(Z, Z, b.size.0, b.size.1);
    ctx.fill().unwrap();
}

