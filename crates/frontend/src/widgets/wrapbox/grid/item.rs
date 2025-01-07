use core::fmt::Debug;

use cairo::ImageSurface;

pub trait GridItemContent {
    fn draw(&mut self) -> ImageSurface;
}

#[derive(Debug)]
pub struct GridItemMap<T: GridItemContent> {
    pub items: Vec<T>,
    // record each row start index in `items`
    pub row_index: Vec<usize>,
}
impl<T: GridItemContent> Default for GridItemMap<T> {
    fn default() -> Self {
        Self {
            items: Vec::default(),
            row_index: Vec::default(),
        }
    }
}
