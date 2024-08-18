use core::fmt::Debug;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use crate::{
    config::widgets::wrapbox::Align,
    ui::{
        draws::{mouse_state::MouseEvent, util::Z},
        widgets::wrapbox::MousePosition,
    },
};
use gtk::gdk::cairo::{self, Format, ImageSurface};

pub trait DisplayWidget {
    fn get_size(&mut self) -> (f64, f64);
    fn content(&mut self) -> ImageSurface;
    fn on_mouse_event(&mut self, _: MouseEvent) {}
}

impl Debug for dyn DisplayWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DisplayWidget")
    }
}

pub type FilteredGridMap = Vec<Vec<BoxedWidgetRc>>;

pub type BoxWidgetIndex = (usize, usize);
pub type BoxedWidgetRc = Rc<RefCell<dyn DisplayWidget>>;
pub type GridMap = Vec<Vec<Option<BoxedWidgetRc>>>;

pub struct GridBox {
    /// first row, second col
    /// [
    ///   [a, b ,c],
    ///   [d, e, f],
    /// ]
    pub ws: GridMap,
    pub row_col_num: (usize, usize),
    pub gap: f64,
    pub align: Align,
}
impl GridBox {
    pub fn new(gap: f64, align: Align) -> Self {
        Self {
            ws: vec![],
            row_col_num: (0, 0),
            gap,
            align,
        }
    }

    pub fn add(
        &mut self,
        w: Rc<RefCell<dyn DisplayWidget + 'static>>,
        position: (isize, isize),
    ) -> BoxWidgetIndex {
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

    pub fn draw_content(&mut self) -> (ImageSurface, GridItemSizeMap) {
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
                let width = &mut size.0;
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

                let pos_x = match self.align {
                    Align::Left => position_x,
                    Align::Center => position_x + (size.0 - content_size.0) / 2.,
                    Align::Right => position_x + (size.0 - content_size.0),
                };
                let pos = (pos_x, position_y + (size.1 - content_size.1) / 2.);
                let content = w.content();

                {
                    ctx.save().unwrap();
                    ctx.translate(pos.0.ceil(), pos.1.ceil());
                    ctx.set_source_surface(content, Z, Z).unwrap();
                    ctx.paint().unwrap();
                    ctx.restore().unwrap();
                }

                position_x += col_width + self.gap;
            }

            position_y += row_height + self.gap;
        }

        let gm = GridItemSizeMap::new(map, self.gap, total_size, existed_widgets);
        (surf, gm)
    }
}
impl Drop for GridBox {
    fn drop(&mut self) {
        log::debug!("drop grid box");
    }
}

pub type GridItemSizeMapRc = Rc<Cell<GridItemSizeMap>>;
pub struct GridItemSizeMap {
    map: [Vec<f64>; 2],
    gap: f64,
    total_size: (f64, f64),
    item_map: FilteredGridMap,
}
impl GridItemSizeMap {
    fn new(
        map: [Vec<f64>; 2],
        gap: f64,
        total_size: (f64, f64),
        item_map: FilteredGridMap,
    ) -> Self {
        Self {
            map,
            gap,
            total_size,
            item_map,
        }
    }
    pub fn match_item(&self, pos: MousePosition) -> Option<(BoxedWidgetRc, MousePosition)> {
        if pos.0 < 0. || pos.1 < 0. || pos.0 > self.total_size.0 || pos.1 > self.total_size.1 {
            None
        } else {
            let (row, y) = {
                let mut start = 0.;
                let mut row_idx = None;
                for (i, h) in self.map[0].iter().enumerate() {
                    let s = *h;
                    let y = pos.1 - start;
                    let range = s + self.gap;
                    if y <= s {
                        row_idx = Some((i, y));
                        break;
                    } else if y < range {
                        return None;
                    };

                    start += range;
                }
                row_idx.unwrap()
            };
            let (col, x) = {
                let mut start = 0.;
                let mut col_idx = None;
                for (i, w) in self.map[1].iter().enumerate() {
                    let s = *w;
                    let x = pos.0 - start;
                    let range = s + self.gap;
                    if x <= s {
                        col_idx = Some((i, x));
                        break;
                    } else if x < range {
                        return None;
                    };

                    start += range;
                }
                col_idx.unwrap()
            };
            Some(((row, col), (x, y)))
        }
        .map(|(idx, pos)| {
            let item = self.item_map[idx.0][idx.1].clone();
            (item, pos)
        })
    }
}
