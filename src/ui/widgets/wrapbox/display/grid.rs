use core::fmt::Debug;
use std::{cell::RefCell, rc::Rc, str::FromStr};

use crate::ui::{draws::util::Z, widgets::ring::init_ring};
use gtk::gdk::{
    cairo::{self, Context, Format, ImageSurface},
    RGBA,
};

pub trait DisplayWidget {
    fn get_size(&mut self) -> (f64, f64);
    fn content(&mut self) -> ImageSurface;
}

impl Debug for dyn DisplayWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DisplayWidget")
    }
}

pub type BoxWidgetIndex = (usize, usize);
pub type GridMap = Vec<Vec<Option<Rc<RefCell<dyn DisplayWidget>>>>>;

pub struct BoxWidgets {
    /// first row, second col
    /// [
    ///   [a, b ,c],
    ///   [d, e, f],
    /// ]
    pub ws: GridMap,
    pub row_col_num: (usize, usize),
    pub gap: f64,
}
impl BoxWidgets {
    pub fn new(gap: f64) -> Self {
        Self {
            ws: vec![],
            row_col_num: (0, 0),
            gap,
        }
    }

    pub fn add(
        &mut self,
        w: Rc<RefCell<dyn DisplayWidget + 'static>>,
        position: (isize, isize),
    ) -> (usize, usize) {
        let mut pos: (usize, usize) = (0, 0);
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

        // self.size_change_map.insert(pos, w.get_size());

        {
            while self.ws.len() < pos.0 + 1 {
                self.ws.push(vec![]);
            }
            if self.ws.len() > self.row_col_num.0 {
                self.row_col_num.0 = self.ws.len();
            }
            let row = &mut self.ws[pos.0];
            while row.len() < pos.1 + 1 {
                row.push(None)
            }
            row[pos.1] = Some(w);
            if row.len() > self.row_col_num.1 {
                self.row_col_num.1 = row.len();
            }
        };

        pos
    }

    /// 0 -> each row max height
    /// 1 -> each col max width
    /// return: row_col_max_map, total_size
    pub fn draw_content(&mut self) -> ImageSurface {
        let mut map = [
            Vec::with_capacity(self.row_col_num.0),
            Vec::with_capacity(self.row_col_num.1),
        ];

        fn s(map: &mut [Vec<f64>; 2], pos: (usize, usize), mut size: (f64, f64)) {
            {
                let height = &mut size.1;
                let max_height = {
                    let row_heights = &mut map[0];
                    while row_heights.len() < pos.0 + 1 {
                        row_heights.push(Z);
                    }
                    &mut row_heights[pos.0]
                };
                if max_height < height {
                    *max_height = *height
                }
            }

            {
                let width = &mut size.1;
                let max_width = {
                    let col_widths = &mut map[1];
                    while col_widths.len() < pos.1 + 1 {
                        col_widths.push(Z);
                    }
                    &mut col_widths[pos.1]
                };
                if max_width < width {
                    *max_width = *width
                }
            }
        }

        let mut existed_widgets = Vec::with_capacity(self.row_col_num.0);
        {
            let mut row_idx = 0;
            for row in self.ws.iter_mut() {
                if row.is_empty() {
                    continue;
                }
                let mut existed_widgets_row = Vec::with_capacity(row.len());
                for (col_idx, w) in row.iter_mut().flatten().enumerate() {
                    s(&mut map, (row_idx, col_idx), w.borrow_mut().get_size());
                    existed_widgets_row.push(w.clone());
                }
                existed_widgets.push(existed_widgets_row);
                row_idx += 1;
            }
        }
        let total_size = {
            let mut height = 0.;
            for (i, s) in map[0].iter().enumerate() {
                let s = *s;
                if s > Z {
                    if i != map[0].len() - 1 {
                        height += self.gap;
                    }
                    height += s;
                }
            }

            let mut width = 0.;
            for (i, s) in map[1].iter().enumerate() {
                let s = *s;
                if s > Z {
                    if i != map[1].len() - 1 {
                        width += self.gap;
                    }
                    width += s;
                }
            }
            (width, height)
        };

        let surf = ImageSurface::create(
            Format::ARgb32,
            total_size.0.ceil() as i32,
            total_size.1.ceil() as i32,
        )
        .unwrap();
        let ctx = cairo::Context::new(&surf).unwrap();

        println!("total size: {:?}", total_size);
        println!("map: {:?}", map);
        println!("existed_widgets: {:?}", existed_widgets);

        let mut position_y = 0.;
        for (row_idx, row) in existed_widgets.iter().enumerate() {
            let row_height = map[0][row_idx];

            let mut position_x = 0.;
            for (col_idx, w) in row.iter().enumerate() {
                let col_width = map[1][col_idx];
                let size = (col_width, row_height);
                // let pos = (position_x, position_y);
                let mut w = w.borrow_mut();
                let content_size = w.get_size();
                let pos = (
                    position_x + (size.0 - content_size.0) / 2.,
                    position_y + (size.1 - content_size.1) / 2.,
                );
                let content = w.content();
                println!("pos: {:?}", pos);

                {
                    ctx.save().unwrap();
                    ctx.translate(pos.0, pos.1);
                    ctx.set_source_surface(content, Z, Z).unwrap();
                    ctx.rectangle(Z, Z, content_size.0, content_size.1);
                    ctx.fill().unwrap();
                    ctx.restore().unwrap();
                }

                position_x += col_width + self.gap;
            }

            position_y += row_height + self.gap;
        }

        surf
    }
}

fn draw(ctx: &Context) {
    let map_size = (200., 200.);
    // let map_size = (40., 200.);
    let size_factors = (0.75, 0.85);
    let margins = Some([3., 3., 3., 3.]);
    let mut bws = BoxWidgets::new(10.);
    for i in 0..9 {
        let ring = Rc::new(RefCell::new(init_ring(
            5.,
            5. + i as f64 * 2.,
            RGBA::from_str("#9F9F9F").unwrap(),
            RGBA::from_str("#F1FA8C").unwrap(),
        )));

        let r_idx = i / 3;
        let c_idx = i % 3;
        bws.add(ring, (r_idx, c_idx));
    }
    println!("{:?}", bws.ws);
    let content = bws.draw_content();

    ctx.set_source_surface(&content, Z, Z).unwrap();
    ctx.rectangle(Z, Z, map_size.0, map_size.1);
    ctx.fill().unwrap();

    // let max_size_map = bws.gen_max_row_cols_map();
    // let total_size = (max_size_map[0].iter().sum(), max_size_map[1].iter().sum());
    //
    // let content = { bws.ws.iter_mut() };
    //
    // let border_color = RGBA::from_str("#C18F4A").unwrap();
    // let radius_percentage = 0.3;
    // let b = BoxDrawsCache::new(
    //     content_size,
    //     margins,
    //     border_color,
    //     // Some(RGBA::GREEN),
    //     None,
    //     radius_percentage,
    //     size_factors,
    // );

    // ctx.set_source_surface(b.with_box(content), Z, Z).unwrap();
    // ctx.rectangle(Z, Z, b.size.0, b.size.1);
    // ctx.fill().unwrap();
}
