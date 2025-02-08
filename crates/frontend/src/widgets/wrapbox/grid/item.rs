use core::fmt::Debug;

#[derive(Debug)]
pub struct GridItemMap<T> {
    pub items: Vec<T>,
    // record each row start index in `items`
    pub row_index: Vec<usize>,
}
impl<T> Default for GridItemMap<T> {
    fn default() -> Self {
        Self {
            items: Vec::default(),
            row_index: Vec::default(),
        }
    }
}
