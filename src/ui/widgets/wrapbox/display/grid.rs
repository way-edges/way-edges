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
pub type GridMap<T> = Vec<Vec<Option<T>>>;

type NewMap<T> = Box<[(GridItemIndex, T)]>;
type GridItemIndex = (usize, usize);

pub struct GrideBoxBuilder<T> {
    ws: GridMap<T>,
    row_index_in_map: Vec<usize>,
    row_col_num: (usize, usize),
}
impl<T: Clone> GrideBoxBuilder<T> {
    pub fn new() -> Self {
        Self {
            ws: vec![],
            row_col_num: (0, 0),
            row_index_in_map: vec![],
        }
    }

    pub fn add(&mut self, w: T, position: (isize, isize)) -> BoxWidgetIndex {
        let mut pos: GridItemIndex = (0, 0);

        // row index
        pos.0 = if position.0 == -1 {
            self.row_col_num.0
        } else if position.0 >= 0 {
            position.0 as usize
        } else {
            panic!("position must be positive or -1");
        };

        // col index
        pos.1 = if position.1 == -1 {
            self.row_col_num.1
        } else if position.1 >= 0 {
            position.1 as usize
        } else {
            panic!("position must be positive or -1");
        };

        // self.size_change_map.insert(pos, w.get_size());

        macro_rules! ensure_vec {
            ($vec:expr, $need_size:expr, $update_len:expr, $val:expr) => {
                if $need_size > $vec.len() {
                    $vec.resize_with($need_size, || $val);
                    $update_len = $vec.len()
                }
            };
        }
        // create row if not enough
        let vec = &mut self.ws;
        ensure_vec!(vec, pos.0 + 1, self.row_col_num.0, vec![]);

        // create col if not enough
        let vec = &mut self.ws[pos.0];
        ensure_vec!(vec, pos.1 + 1, self.row_col_num.1, None);

        vec[pos.1] = Some(w);

        pos
    }

    pub fn build(self, gap: f64, align: Align) -> GridBox<T> {
        let mut map = vec![];
        for (i, row) in self.ws.into_iter().enumerate() {
            for (j, col) in row.into_iter().enumerate() {
                if let Some(w) = col {
                    map.push(((i, j), w));
                }
            }
        }

        let mut row_col_num = (0, 0);
        map.iter().for_each(|((i, j), _)| {
            row_col_num.0 = *i + 1;
            row_col_num.1 = row_col_num.1.max(*j + 1);
        });

        let map = map.into_boxed_slice();

        macro_rules! align_y {
            (T) => {
                pos.1
            };
            (C) => {
                pos.1 + (size.1 - content_size.1) / 2.
            };
            (B) => {
                pos.1 + (size.1 - content_size.1)
            };
        }

        macro_rules! align_x {
            (L, $pos:expr) => {
                $pos.0
            };
            (C) => {
                pos.0 + (size.0 - content_size.0) / 2.
            };
            (R) => {
                pos.0 + (size.0 - content_size.0)
            };
        }

        macro_rules! a {
            ($x:tt, $y:tt) => {
                |pos, size, content_size| (align_x!($x, pos), align_y!($y))
            };
        }

        let align_func: AlignFunc = Box::new(match align {
            // Align::Left => |posx, _, _| posx,
            // Align::Center => |posx, size, content_size| posx + (size.0 - content_size.0) / 2.,
            // Align::Right => |posx, size, content_size| posx + (size.0 - content_size.0),
            Align::TopLeft => a!(L, T),
            Align::TopCenter => todo!(),
            Align::TopRight => todo!(),
            Align::CenterLeft => todo!(),
            Align::CenterCenter => todo!(),
            Align::CenterRight => todo!(),
            Align::BottomLeft => todo!(),
            Align::BottomCenter => todo!(),
            Align::BottomRight => todo!(),
        });

        GridBox {
            map,
            row_col_num,
            gap,
            align_func,
        }
    }
}

type AlignFunc = Box<fn(f64, (f64, f64), (f64, f64)) -> f64>;

pub struct GridBox<T> {
    pub map: NewMap<T>,
    pub row_col_num: (usize, usize),
    pub gap: f64,
    pub align_func: AlignFunc,
}
impl GridBox<BoxedWidgetRc> {
    pub fn draw_content(&mut self) -> (ImageSurface, GridItemSizeMap) {
        let mut map = [
            // height of each row
            vec![0.; self.row_col_num.0],
            // width of each col
            vec![0.; self.row_col_num.1],
        ];

        self.map.iter().for_each(|((i, j), widget)| {
            // TODO: CHANGE SIZE TYPE TO I32
            let widget_content_size = widget.borrow().get_size();

            let height: &mut f64 = &mut map[0][*i];
            *height = height.max(widget_content_size.1);

            let width: &mut f64 = &mut map[1][*j];
            *width = width.max(widget_content_size.0);
        });

        fn join_size(v: &Vec<f64>, gap: f64) -> f64 {
            let mut m = 0.;
            for (i, s) in v.iter().enumerate() {
                if i == 0 {
                    m += *s;
                } else {
                    m += gap + *s;
                }
            }
            m
        }

        let total_size = (
            join_size(&map[1].clone(), self.gap),
            join_size(&map[0].clone(), self.gap),
        );

        let surf = ImageSurface::create(
            Format::ARgb32,
            total_size.0.ceil() as i32,
            total_size.1.ceil() as i32,
        )
        .unwrap();
        let ctx = cairo::Context::new(&surf).unwrap();

        let mut position_x = 0.;
        let mut position_y = 0.;
        let mut i_cache = 0;
        let mut j_cache = 0;
        self.map.iter().for_each(|((i, j), widget)| {
            let surf = widget.borrow().content();

            if *j < j_cache {
                // lower mean going to a new row
                position_x = 0.;
            } else if *j > j_cache {
                // biggre means next element in the same row
                position_x += map[1][j_cache] + self.gap;
            }

            // i shall only get bigger
            if *i > i_cache {
                // new row
                position_y += map[0][i_cache] + self.gap;
            }

            j_cache = *j;
            i_cache = *i;
        });

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

                ctx.save().unwrap();
                ctx.translate(pos.0.ceil(), pos.1.ceil());
                ctx.set_source_surface(content, Z, Z).unwrap();
                ctx.paint().unwrap();
                ctx.restore().unwrap();

                position_x += col_width + self.gap;
            }

            position_y += row_height + self.gap;
        }

        let gm = GridItemSizeMap::new(map, self.gap, total_size, existed_widgets);
        (surf, gm)
    }
}

// pub struct GridBox {
//     /// first row, second col
//     /// [
//     ///   [a, b ,c],
//     ///   [d, e, f],
//     /// ]
//     pub ws: GridMap,
//     pub row_col_num: (usize, usize),
//     pub gap: f64,
//     pub align: Align,
// }
// impl GridBox {
//     pub fn new(gap: f64, align: Align) -> Self {
//         Self {
//             ws: vec![],
//             row_col_num: (0, 0),
//             gap,
//             align,
//         }
//     }
//
//     pub fn add(
//         &mut self,
//         w: Rc<RefCell<dyn DisplayWidget + 'static>>,
//         position: (isize, isize),
//     ) -> BoxWidgetIndex {
//         let mut pos: (usize, usize) = (0, 0);
//         pos.0 = if position.0 == -1 {
//             self.row_col_num.0
//         } else if position.0 >= 0 {
//             position.0 as usize
//         } else {
//             panic!("position must be positive or -1");
//         };
//         pos.1 = if position.1 == -1 {
//             self.row_col_num.1
//         } else if position.1 >= 0 {
//             position.1 as usize
//         } else {
//             panic!("position must be positive or -1");
//         };
//
//         // self.size_change_map.insert(pos, w.get_size());
//
//         {
//             while self.ws.len() < pos.0 + 1 {
//                 self.ws.push(vec![]);
//             }
//             if self.ws.len() > self.row_col_num.0 {
//                 self.row_col_num.0 = self.ws.len();
//             }
//             let row = &mut self.ws[pos.0];
//             while row.len() < pos.1 + 1 {
//                 row.push(None)
//             }
//             row[pos.1] = Some(w);
//             if row.len() > self.row_col_num.1 {
//                 self.row_col_num.1 = row.len();
//             }
//         };
//
//         pos
//     }
//
//     pub fn draw_content(&mut self) -> (ImageSurface, GridItemSizeMap) {
//         let mut map = [
//             Vec::with_capacity(self.row_col_num.0),
//             Vec::with_capacity(self.row_col_num.1),
//         ];
//
//         fn s(map: &mut [Vec<f64>; 2], pos: (usize, usize), mut size: (f64, f64)) {
//             {
//                 let height = &mut size.1;
//                 let max_height = {
//                     let row_heights = &mut map[0];
//                     while row_heights.len() < pos.0 + 1 {
//                         row_heights.push(Z);
//                     }
//                     &mut row_heights[pos.0]
//                 };
//                 if max_height < height {
//                     *max_height = *height
//                 }
//             }
//
//             {
//                 let width = &mut size.0;
//                 let max_width = {
//                     let col_widths = &mut map[1];
//                     while col_widths.len() < pos.1 + 1 {
//                         col_widths.push(Z);
//                     }
//                     &mut col_widths[pos.1]
//                 };
//                 if max_width < width {
//                     *max_width = *width
//                 }
//             }
//         }
//
//         let mut existed_widgets = Vec::with_capacity(self.row_col_num.0);
//         {
//             let mut row_idx = 0;
//             for row in self.ws.iter_mut() {
//                 if row.is_empty() {
//                     continue;
//                 }
//                 let mut existed_widgets_row = Vec::with_capacity(row.len());
//                 for (col_idx, w) in row.iter_mut().flatten().enumerate() {
//                     s(&mut map, (row_idx, col_idx), w.borrow_mut().get_size());
//                     existed_widgets_row.push(w.clone());
//                 }
//                 existed_widgets.push(existed_widgets_row);
//                 row_idx += 1;
//             }
//         }
//         let total_size = {
//             let mut height = 0.;
//             for (i, s) in map[0].iter().enumerate() {
//                 let s = *s;
//                 if s > Z {
//                     if i != map[0].len() - 1 {
//                         height += self.gap;
//                     }
//                     height += s;
//                 }
//             }
//
//             let mut width = 0.;
//             for (i, s) in map[1].iter().enumerate() {
//                 let s = *s;
//                 if s > Z {
//                     if i != map[1].len() - 1 {
//                         width += self.gap;
//                     }
//                     width += s;
//                 }
//             }
//             (width, height)
//         };
//
//         let surf = ImageSurface::create(
//             Format::ARgb32,
//             total_size.0.ceil() as i32,
//             total_size.1.ceil() as i32,
//         )
//         .unwrap();
//         let ctx = cairo::Context::new(&surf).unwrap();
//
//         let mut position_y = 0.;
//         for (row_idx, row) in existed_widgets.iter().enumerate() {
//             let row_height = map[0][row_idx];
//
//             let mut position_x = 0.;
//             for (col_idx, w) in row.iter().enumerate() {
//                 let col_width = map[1][col_idx];
//                 let size = (col_width, row_height);
//                 // let pos = (position_x, position_y);
//                 let mut w = w.borrow_mut();
//                 let content_size = w.get_size();
//
//                 let pos_x = match self.align {
//                     Align::Left => position_x,
//                     Align::Center => position_x + (size.0 - content_size.0) / 2.,
//                     Align::Right => position_x + (size.0 - content_size.0),
//                 };
//                 let pos = (pos_x, position_y + (size.1 - content_size.1) / 2.);
//                 let content = w.content();
//
//                 ctx.save().unwrap();
//                 ctx.translate(pos.0.ceil(), pos.1.ceil());
//                 ctx.set_source_surface(content, Z, Z).unwrap();
//                 ctx.paint().unwrap();
//                 ctx.restore().unwrap();
//
//                 position_x += col_width + self.gap;
//             }
//
//             position_y += row_height + self.gap;
//         }
//
//         let gm = GridItemSizeMap::new(map, self.gap, total_size, existed_widgets);
//         (surf, gm)
//     }
// }
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
