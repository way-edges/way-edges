use core::fmt::Debug;

use cairo::ImageSurface;

use crate::buffer::Buffer;

#[derive(Debug)]
pub(super) struct GridItem<T> {
    item: T,
    buffer: Buffer,
}
impl<T> GridItem<T> {
    pub(super) fn new(t: T) -> Self {
        Self {
            item: t,
            buffer: Buffer::default(),
        }
    }
    pub(super) fn get_item(&self) -> &T {
        &self.item
    }
    pub(super) fn update_buffer(&mut self, img: ImageSurface) {
        self.buffer.update_buffer(img);
    }
    pub(super) fn get_buffer(&self) -> ImageSurface {
        self.buffer.get_buffer()
    }
}

#[derive(Debug)]
pub(super) struct GridItemMap<T> {
    pub(super) items: Vec<GridItem<T>>,
    // record each row start index in `items`
    pub(super) row_index: Vec<usize>,
}
impl<T> Default for GridItemMap<T> {
    fn default() -> Self {
        Self {
            items: Vec::default(),
            row_index: Vec::default(),
        }
    }
}
