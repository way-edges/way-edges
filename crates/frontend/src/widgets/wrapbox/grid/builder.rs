use config::def::widgets::wrapbox::{Align, AlignFunc};

use super::{item::GridItemMap, GridBox};

pub struct GrideBoxBuilder<T> {
    ws: Vec<Vec<Option<T>>>,
    row_col_num: (usize, usize),
}
impl<T> GrideBoxBuilder<T> {
    pub fn new() -> Self {
        Self {
            ws: vec![],
            row_col_num: (0, 0),
        }
    }

    pub fn add(&mut self, w: T, position: (isize, isize)) -> (usize, usize) {
        let mut pos = (0, 0);

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
        let align_func: AlignFunc = align.to_func();

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
            item_map: grid_item_map,
            row_col_num,
            gap,
            align_func,
            position_map: None,
        }
    }
}
