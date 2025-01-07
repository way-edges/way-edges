use cairo::ImageSurface;
use util::draw::new_surface;

#[derive(Clone, Debug, Default)]
pub struct Buffer(Option<ImageSurface>);
impl Buffer {
    pub fn update_buffer(&mut self, new: ImageSurface) {
        self.0.replace(new);
    }
    pub fn get_buffer(&self) -> ImageSurface {
        self.0.clone().unwrap_or(new_surface((0, 0)))
    }
}
