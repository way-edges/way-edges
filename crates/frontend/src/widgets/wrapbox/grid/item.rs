use core::fmt::Debug;

use cairo::ImageSurface;

pub trait GridItemContent {
    fn draw(&mut self) -> ImageSurface;
    fn has_update(&mut self) -> bool;
}

#[derive(Debug)]
pub(super) struct GridItemMap<T: GridItemContent> {
    pub(super) items: Vec<T>,
    // record each row start index in `items`
    pub(super) row_index: Vec<usize>,
}
impl<T: GridItemContent> Default for GridItemMap<T> {
    fn default() -> Self {
        Self {
            items: Vec::default(),
            row_index: Vec::default(),
        }
    }
}
