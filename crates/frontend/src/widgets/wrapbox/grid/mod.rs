pub mod builder;
mod item;

use config::widgets::wrapbox::{Align, AlignFunc};
use gtk::gdk::cairo::{self, Format, ImageSurface};
use item::GridItemMap;
use util::binary_search_within_range;

struct GridPositionMap {
    // use i32 to save memory
    total_size: (i32, i32),
    grid_cell_position_map: [Vec<[i32; 2]>; 2],
    widget_start_point_list: Vec<(f64, f64)>,
}
impl GridPositionMap {
    fn match_item<'a, T>(
        &self,
        pos: (f64, f64),
        item_map: &'a item::GridItemMap<T>,
    ) -> Option<(&'a T, (f64, f64))> {
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

        let widget_index = item_map.row_index[which_row as usize] + which_col as usize;

        let start_point = self.widget_start_point_list.get(widget_index)?;
        let new_position = (pos.0 - start_point.0, pos.1 - start_point.1);
        let widget = &item_map.items[widget_index];

        Some((widget.get_item(), new_position))
    }
}

pub struct GridBox<T> {
    position_map: Option<GridPositionMap>,
    item_map: GridItemMap<T>,
    pub row_col_num: (usize, usize),
    pub gap: f64,
    pub align_func: AlignFunc,
}
impl<T> GridBox<T> {
    pub fn new(gap: f64, align: Align) -> Self {
        Self {
            item_map: GridItemMap::default(),
            row_col_num: (0, 0),
            gap,
            align_func: align.to_func(),
            position_map: None,
        }
    }
    pub fn match_item(&self, pos: (f64, f64)) -> Option<(&T, (f64, f64))> {
        self.position_map
            .as_ref()
            .and_then(|position_map| position_map.match_item(pos, &self.item_map))
    }
    pub fn draw(&mut self, get_content_func: impl Fn(&T) -> Option<ImageSurface>) -> ImageSurface {
        if self.item_map.row_index.is_empty() {
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
            let max_row = self.item_map.row_index.len() - 1;
            self.item_map
                .items
                .iter_mut()
                .enumerate()
                .for_each(|(widget_index, widget)| {
                    // ensure in the correct row
                    if which_row != max_row {
                        // if reaches next row
                        if widget_index == self.item_map.row_index[next_row] {
                            which_row = next_row;
                            next_row += 1;
                        }
                    }

                    // calculate col index
                    let current_row_start_index = self.item_map.row_index[which_row];
                    let which_col = widget_index - current_row_start_index;

                    // get content
                    if let Some(img) = get_content_func(widget.get_item()) {
                        widget.update_buffer(img)
                    }
                    let content = widget.get_buffer();

                    // calculate size
                    let widget_content_size = (content.width() as f64, content.height() as f64);
                    // max height for each row
                    let height: &mut f64 = &mut grid_block_size_map[0][which_row];
                    *height = height.max(widget_content_size.1);
                    // max width for each col
                    let width: &mut f64 = &mut grid_block_size_map[1][which_col];
                    *width = width.max(widget_content_size.0);

                    // put into render map
                    widget_render_map[which_row].push(content);
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

            for (which_col, surf) in row.into_iter().enumerate() {
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
        });
        surf
    }
}
