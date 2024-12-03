use core::fmt::Debug;
use std::{cell::RefCell, rc::Rc};

use crate::{
    common::binary_search_within_range, config::widgets::wrapbox::Align,
    ui::draws::mouse_state::MouseEvent,
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

pub type BoxWidgetIndex = (usize, usize);
pub type BoxedWidgetRc = Rc<RefCell<dyn DisplayWidget>>;
pub type GridMap<T> = Vec<Vec<Option<T>>>;

pub struct GridItemMap<T> {
    items: Vec<T>,
    // record each row start index in `items`
    row_index: Vec<usize>,
}

type GridItemIndex = (usize, usize);

pub struct GrideBoxBuilder<T> {
    ws: GridMap<T>,
    row_col_num: (usize, usize),
}
impl<T: Clone> GrideBoxBuilder<T> {
    pub fn new() -> Self {
        Self {
            ws: vec![],
            row_col_num: (0, 0),
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
        macro_rules! align_y {
            (T, $pos:expr, $size:expr, $content_size:expr) => {
                $pos.1
            };
            (C, $pos:expr, $size:expr, $content_size:expr) => {
                $pos.1 + ($size.1 - $content_size.1) / 2.
            };
            (B, $pos:expr, $size:expr, $content_size:expr) => {
                $pos.1 + ($size.1 - $content_size.1)
            };
        }

        macro_rules! align_x {
            (L, $pos:expr, $size:expr, $content_size:expr) => {
                $pos.0
            };
            (C, $pos:expr, $size:expr, $content_size:expr) => {
                $pos.0 + ($size.0 - $content_size.0) / 2.
            };
            (R, $pos:expr, $size:expr, $content_size:expr) => {
                $pos.0 + ($size.0 - $content_size.0)
            };
        }

        macro_rules! a {
            ($x:tt $y:tt) => {
                |pos, size, content_size| {
                    (
                        align_x!($x, pos, size, content_size),
                        align_y!($y, pos, size, content_size),
                    )
                }
            };
        }

        let align_func: AlignFunc = Box::new(match align {
            #[allow(unused)]
            Align::TopLeft => a!(L T),
            Align::TopCenter => a!(C T),
            Align::TopRight => a!(R T),
            Align::CenterLeft => a!(L C),
            Align::CenterCenter => a!(C C),
            Align::CenterRight => a!(R C),
            Align::BottomLeft => a!(L B),
            Align::BottomCenter => a!(C B),
            Align::BottomRight => a!(R B),
        });

        let mut items = vec![];
        let mut row_index = vec![];

        let mut index = 0;
        let mut max_col = 0;
        // filter the emptys
        for row in self.ws.into_iter() {
            let mut col_index = 0;
            for widget in row.into_iter().flatten() {
                col_index += 1;
                items.push(widget);
            }

            if col_index > 0 {
                row_index.push(index);
                max_col = max_col.max(col_index);
                index += col_index;
            }
        }

        let row_col_num = (row_index.len(), max_col);
        let grid_item_map = GridItemMap { items, row_index };

        GridBox {
            map: grid_item_map,
            row_col_num,
            gap,
            align_func,
            position_map: None,
        }
    }
}

type AlignFuncPos = (f64, f64);
type AlignFuncGridBlockSize = (f64, f64);
type AlignFuncContentSize = (f64, f64);
type AlignFunc =
    Box<fn(AlignFuncPos, AlignFuncGridBlockSize, AlignFuncContentSize) -> AlignFuncPos>;

pub struct GridPositionMap<T> {
    // use i32 to save memory
    total_size: (i32, i32),

    grid_cell_position_map: [Vec<[i32; 2]>; 2],
    widget_start_point_list: Vec<(f64, f64)>,

    grid_item_map: *const GridItemMap<T>,
}
impl<T> GridPositionMap<T> {
    pub fn match_item(&self, pos: (f64, f64)) -> Option<(&T, (f64, f64))> {
        if pos.0 < 0.
            || pos.1 < 0.
            || pos.0 > self.total_size.0 as f64
            || pos.1 > self.total_size.1 as f64
        {
            return None;
        }

        let which_row = binary_search_within_range(&self.grid_cell_position_map[0], pos.1 as i32);
        let which_col = binary_search_within_range(&self.grid_cell_position_map[1], pos.0 as i32);

        if which_row == -1 || which_col == -1 {
            return None;
        }

        let item_map = unsafe { self.grid_item_map.as_ref() }.unwrap();
        let widget_index = item_map.row_index[which_row as usize] + which_col as usize;

        let start_point = self.widget_start_point_list[widget_index];
        let new_position = (pos.0 - start_point.0, pos.1 - start_point.1);
        let widget = &item_map.items[widget_index];

        Some((widget, new_position))
    }
}

pub struct GridBox<T> {
    // pub map: NewMap<T>,
    pub map: GridItemMap<T>,
    pub row_col_num: (usize, usize),
    pub gap: f64,
    pub align_func: AlignFunc,

    pub position_map: Option<GridPositionMap<T>>,
}
impl GridBox<BoxedWidgetRc> {
    pub fn draw_content(&mut self) -> ImageSurface {
        if self.map.row_index.is_empty() {
            return ImageSurface::create(Format::ARgb32, 0, 0).unwrap();
        }

        let (grid_block_size_map, widget_render_map) = {
            let mut grid_block_size_map = [
                // height of each row
                vec![0.; self.row_col_num.0],
                // width of each col
                vec![0.; self.row_col_num.1],
            ];

            let mut widget_render_map =
                vec![Vec::with_capacity(self.row_col_num.1); self.row_col_num.0];

            let mut which_row = 0;
            let mut next_row = which_row + 1;
            let max_row = self.map.row_index.len() - 1;
            self.map
                .items
                .iter()
                .enumerate()
                .for_each(|(widget_index, widget)| {
                    // ensure in the correct row
                    if which_row != max_row {
                        // if reaches next row
                        if widget_index == self.map.row_index[next_row] {
                            which_row = next_row;
                            next_row += 1;
                        }
                    }

                    // calculate col index
                    let current_row_start_index = self.map.row_index[which_row];
                    let which_col = widget_index - current_row_start_index;

                    // put into render map
                    widget_render_map[which_row].push(widget);

                    // calculate size
                    let widget_content_size = widget.borrow_mut().get_size();
                    // max height for each row
                    let height: &mut f64 = &mut grid_block_size_map[0][which_row];
                    *height = height.max(widget_content_size.1);
                    // max width for each col
                    let width: &mut f64 = &mut grid_block_size_map[1][which_col];
                    *width = width.max(widget_content_size.0);
                });

            (grid_block_size_map, widget_render_map)
        };

        let total_size = {
            #[inline]
            fn join_size(v: &[f64], gap: f64) -> f64 {
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

            (
                join_size(&grid_block_size_map[1].clone(), self.gap).ceil() as i32,
                join_size(&grid_block_size_map[0].clone(), self.gap).ceil() as i32,
            )
        };

        let surf = ImageSurface::create(Format::ARgb32, total_size.0, total_size.1).unwrap();
        let ctx = cairo::Context::new(&surf).unwrap();

        let mut widget_start_point_list: Vec<(f64, f64)> = vec![];

        let mut position_y = 0.;
        for (which_row, row) in widget_render_map.into_iter().enumerate() {
            let mut position_x = 0.;

            let max_col = row.len() - 1;

            for (which_col, widget) in row.into_iter().enumerate() {
                let surf = widget.borrow_mut().content();
                let content_size = (surf.width() as f64, surf.height() as f64);

                // calculate start position considering align
                let mut pos = (self.align_func)(
                    (position_x, position_y),
                    // grid cell size
                    (
                        grid_block_size_map[1][which_col],
                        grid_block_size_map[0][which_row],
                    ),
                    content_size,
                );
                pos.0 = pos.0.floor();
                pos.1 = pos.1.floor();

                // push widget's start point
                widget_start_point_list.push(pos);

                // draw
                ctx.set_source_surface(surf, pos.0, pos.1).unwrap();
                ctx.paint().unwrap();

                // add position x
                if which_col < max_col {
                    position_x += grid_block_size_map[1][which_col] + self.gap;
                }
            }

            position_y += grid_block_size_map[0][which_row] + self.gap;
        }

        let grid_cell_position_map = {
            let mut grid_cell_position_map: [Vec<[i32; 2]>; 2] = [
                // y position range(height) of each row
                vec![[0, 0]; self.row_col_num.0],
                // x position range(width) of each col
                vec![[0, 0]; self.row_col_num.1],
            ];
            macro_rules! calculate_grid_cell_position_map {
                ($size_map:expr, $position_map:expr, $gap:expr) => {
                    let mut pos = 0;
                    $size_map.iter().enumerate().for_each(|(i, size)| {
                        let end = pos + *size as i32;
                        $position_map[i] = [pos, end];
                        pos = end + $gap as i32;
                    });
                };
            }
            calculate_grid_cell_position_map!(
                grid_block_size_map[0],
                grid_cell_position_map[0],
                self.gap
            );
            calculate_grid_cell_position_map!(
                grid_block_size_map[1],
                grid_cell_position_map[1],
                self.gap
            );
            grid_cell_position_map
        };

        self.position_map = Some(GridPositionMap {
            total_size,
            grid_cell_position_map,
            widget_start_point_list,
            grid_item_map: &raw const self.map,
        });
        surf
    }
}

impl<T> Drop for GridBox<T> {
    fn drop(&mut self) {
        log::debug!("drop grid box");
    }
}

// pub type GridItemSizeMapRc = Rc<Cell<GridItemSizeMap>>;
// pub struct GridItemSizeMap {
//     map: [Vec<f64>; 2],
//     gap: f64,
//     total_size: (f64, f64),
//     item_map: FilteredGridMap,
// }
// impl GridItemSizeMap {
//     fn new(
//         map: [Vec<f64>; 2],
//         gap: f64,
//         total_size: (f64, f64),
//         item_map: FilteredGridMap,
//     ) -> Self {
//         Self {
//             map,
//             gap,
//             total_size,
//             item_map,
//         }
//     }
//     pub fn match_item(&self, pos: MousePosition) -> Option<(BoxedWidgetRc, MousePosition)> {
//         if pos.0 < 0. || pos.1 < 0. || pos.0 > self.total_size.0 || pos.1 > self.total_size.1 {
//             None
//         } else {
//             let (row, y) = {
//                 let mut start = 0.;
//                 let mut row_idx = None;
//                 for (i, h) in self.map[0].iter().enumerate() {
//                     let s = *h;
//                     let y = pos.1 - start;
//                     let range = s + self.gap;
//                     if y <= s {
//                         row_idx = Some((i, y));
//                         break;
//                     } else if y < range {
//                         return None;
//                     };
//
//                     start += range;
//                 }
//                 row_idx.unwrap()
//             };
//             let (col, x) = {
//                 let mut start = 0.;
//                 let mut col_idx = None;
//                 for (i, w) in self.map[1].iter().enumerate() {
//                     let s = *w;
//                     let x = pos.0 - start;
//                     let range = s + self.gap;
//                     if x <= s {
//                         col_idx = Some((i, x));
//                         break;
//                     } else if x < range {
//                         return None;
//                     };
//
//                     start += range;
//                 }
//                 col_idx.unwrap()
//             };
//             Some(((row, col), (x, y)))
//         }
//         .map(|(idx, pos)| {
//             let item = self.item_map[idx.0][idx.1].clone();
//             (item, pos)
//         })
//     }
// }
